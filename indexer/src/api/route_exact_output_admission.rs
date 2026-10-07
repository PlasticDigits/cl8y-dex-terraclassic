//! #1389: one active receive-sizing request per socket-peer IP, four globally.
//! No queue and no retained idle-client identities. Forwarded headers are untrusted.

use std::{collections::HashSet, future::Future, net::IpAddr, sync::Mutex, time::Duration};

use axum::{extract::ConnectInfo, http::StatusCode, Extension};
use std::net::SocketAddr;

type Error = (StatusCode, String);
pub(super) const MAX_CONCURRENT_REQUESTS: usize = 4;

/// Missing connection metadata shares one conservative bucket; it never disables limits.
/// Normalize IPv4-mapped IPv6 so address spelling cannot buy another share.
pub(super) fn client_key(peer: Option<Extension<ConnectInfo<SocketAddr>>>) -> Option<IpAddr> {
    peer.map(|Extension(ConnectInfo(addr))| match addr.ip() {
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ip)),
        ip => ip,
    })
}

#[derive(Default)]
struct Capacity {
    active: HashSet<Option<IpAddr>>,
    admitted: u64,
    rejected_client: u64,
    rejected_global: u64,
    completed: u64,
    failed: u64,
    timed_out: u64,
    cancelled: u64,
}

impl Capacity {
    /// Aggregate event counters are suitable for log-derived metrics, without IP labels.
    fn record(&self) {
        tracing::info!(target: "exact_output_capacity",
            active = self.active.len(), admitted = self.admitted,
            rejected_client = self.rejected_client, rejected_global = self.rejected_global,
            completed = self.completed, failed = self.failed,
            timed_out = self.timed_out, cancelled = self.cancelled,
            "exact-output admission metrics");
    }
}

#[derive(Default)]
pub(super) struct AdmissionController(Mutex<Capacity>);

enum Outcome {
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

struct Admission<'a> {
    controller: &'a AdmissionController,
    client: Option<IpAddr>,
    outcome: Outcome,
}

impl Drop for Admission<'_> {
    fn drop(&mut self) {
        let mut capacity = self.controller.0.lock().unwrap_or_else(|e| e.into_inner());
        capacity.active.remove(&self.client);
        let counter = match self.outcome {
            Outcome::Completed => &mut capacity.completed,
            Outcome::Failed => &mut capacity.failed,
            Outcome::TimedOut => &mut capacity.timed_out,
            Outcome::Cancelled => &mut capacity.cancelled,
        };
        *counter = counter.saturating_add(1);
        capacity.record();
    }
}

impl AdmissionController {
    fn admit(&self, client: Option<IpAddr>) -> Result<Admission<'_>, Error> {
        let mut capacity = self.0.lock().unwrap_or_else(|e| e.into_inner());
        // Check the client first: repeated requests cannot consume remaining global shares.
        if capacity.active.contains(&client) {
            capacity.rejected_client = capacity.rejected_client.saturating_add(1);
            capacity.record();
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Exact-output client capacity is busy; retry shortly".into(),
            ));
        }
        if capacity.active.len() >= MAX_CONCURRENT_REQUESTS {
            capacity.rejected_global = capacity.rejected_global.saturating_add(1);
            capacity.record();
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Exact-output sizing is busy; retry shortly".into(),
            ));
        }
        capacity.active.insert(client);
        capacity.admitted = capacity.admitted.saturating_add(1);
        capacity.record();
        Ok(Admission {
            controller: self,
            client,
            outcome: Outcome::Cancelled,
        })
    }

    pub(super) async fn run<T>(
        &self,
        client: Option<IpAddr>,
        timeout: Duration,
        work: impl Future<Output = Result<T, Error>>,
    ) -> Result<T, Error> {
        let mut admission = self.admit(client)?;
        match tokio::time::timeout(timeout, work).await {
            Ok(result) => {
                admission.outcome = if result.is_ok() {
                    Outcome::Completed
                } else {
                    Outcome::Failed
                };
                result
            }
            Err(_) => {
                admission.outcome = Outcome::TimedOut;
                Err((
                    StatusCode::GATEWAY_TIMEOUT,
                    "Exact-output sizing timed out; retry".into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::State,
        http::{HeaderValue, Request},
        routing::get,
        Router,
    };
    use std::sync::Arc;
    use tokio::sync::{oneshot, Notify};
    use tower::ServiceExt;

    fn ip(n: u8) -> Option<IpAddr> {
        Some(IpAddr::from([203, 0, 113, n]))
    }

    #[test]
    fn route_exact_output_admission_reserves_distinct_client_shares_and_bounds_state() {
        let controller = AdmissionController::default();
        let first = controller.admit(ip(1)).unwrap();
        for _ in 0..100 {
            assert_eq!(
                controller.admit(ip(1)).err().unwrap().0,
                StatusCode::TOO_MANY_REQUESTS
            );
        }
        let others: Vec<_> = (2..=4).map(|n| controller.admit(ip(n)).unwrap()).collect();
        assert_eq!(
            controller.admit(ip(5)).err().unwrap().0,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            controller.0.lock().unwrap().active.len(),
            MAX_CONCURRENT_REQUESTS
        );
        {
            let capacity = controller.0.lock().unwrap();
            assert_eq!(
                (capacity.rejected_client, capacity.rejected_global),
                (100, 1)
            );
        }
        drop(first);
        let recovered = controller.admit(ip(5)).unwrap();
        drop(others);
        drop(recovered);
        assert!(controller.0.lock().unwrap().active.is_empty());
    }

    #[tokio::test]
    async fn route_exact_output_admission_rejected_work_is_never_polled_and_missing_peer_is_bounded(
    ) {
        let controller = AdmissionController::default();
        let held = controller.admit(None).unwrap();
        let result = controller
            .run(None, Duration::from_secs(1), async {
                panic!("rejected work must never run");
                #[allow(unreachable_code)]
                Ok(())
            })
            .await;
        assert_eq!(result.unwrap_err().0, StatusCode::TOO_MANY_REQUESTS);
        // Missing metadata shares one bucket, leaving actual peers their own shares.
        let actual_peer = controller.admit(ip(1)).unwrap();
        drop(held);
        drop(actual_peer);
        assert!(controller
            .run(None, Duration::from_secs(1), async { Ok(()) })
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn route_exact_output_admission_repeated_cancellation_releases_capacity() {
        let controller = Arc::new(AdmissionController::default());
        for _ in 0..20 {
            let cloned = controller.clone();
            let (started, ready) = oneshot::channel();
            let task = tokio::spawn(async move {
                cloned
                    .run(ip(1), Duration::from_secs(10), async {
                        started.send(()).unwrap();
                        std::future::pending::<Result<(), Error>>().await
                    })
                    .await
            });
            ready.await.unwrap();
            assert!(controller.admit(ip(1)).is_err());
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
            assert!(controller.0.lock().unwrap().active.is_empty());
        }
        assert_eq!(controller.0.lock().unwrap().cancelled, 20);
        assert!(controller
            .run(ip(1), Duration::from_secs(1), async { Ok(()) })
            .await
            .is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn route_exact_output_admission_timeout_and_failure_recover_and_record_metrics() {
        let controller = AdmissionController::default();
        let result = controller
            .run(
                ip(1),
                Duration::from_secs(20),
                std::future::pending::<Result<(), Error>>(),
            )
            .await;
        assert_eq!(result.unwrap_err().0, StatusCode::GATEWAY_TIMEOUT);
        assert!(controller
            .run(ip(1), Duration::from_secs(1), async {
                Err::<(), _>((StatusCode::UNPROCESSABLE_ENTITY, "infeasible".into()))
            })
            .await
            .is_err());
        assert!(controller
            .run(ip(1), Duration::from_secs(1), async { Ok(()) })
            .await
            .is_ok());
        let capacity = controller.0.lock().unwrap();
        assert!(capacity.active.is_empty());
        assert_eq!(
            (
                capacity.admitted,
                capacity.timed_out,
                capacity.failed,
                capacity.completed
            ),
            (3, 1, 1, 1)
        );
    }

    #[tokio::test]
    async fn route_exact_output_admission_ignores_forwarded_headers_and_ports() {
        let controller = Arc::new(AdmissionController::default());
        let held = controller.admit(ip(1)).unwrap();
        async fn endpoint(
            State(controller): State<Arc<AdmissionController>>,
            peer: Option<Extension<ConnectInfo<SocketAddr>>>,
        ) -> Result<(), Error> {
            controller
                .run(client_key(peer), Duration::from_secs(1), async { Ok(()) })
                .await
        }
        let router = Router::new()
            .route("/", get(endpoint))
            .with_state(controller.clone());
        for peer in [
            "203.0.113.1:1000",
            "203.0.113.1:9999",
            "[::ffff:203.0.113.1]:9999",
        ] {
            let mut request = Request::builder()
                .uri("/?trader=spoofed")
                .body(axum::body::Body::empty())
                .unwrap();
            request
                .extensions_mut()
                .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
            for header in [
                "forwarded",
                "x-forwarded-for",
                "cf-connecting-ip",
                "x-real-ip",
            ] {
                request
                    .headers_mut()
                    .insert(header, HeaderValue::from_static("198.51.100.99"));
            }
            assert_eq!(
                router.clone().oneshot(request).await.unwrap().status(),
                StatusCode::TOO_MANY_REQUESTS
            );
        }
        drop(held);
        assert_eq!(
            router
                .oneshot(
                    Request::builder()
                        .uri("/")
                        .body(axum::body::Body::empty())
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn route_exact_output_admission_concurrent_clients_receive_separate_shares() {
        let controller = Arc::new(AdmissionController::default());
        let release = Arc::new(Notify::new());
        let mut tasks = Vec::new();
        for n in 1..=4 {
            let controller = controller.clone();
            let release = release.clone();
            let (started, ready) = oneshot::channel();
            tasks.push(tokio::spawn(async move {
                controller
                    .run(ip(n), Duration::from_secs(10), async {
                        started.send(()).unwrap();
                        release.notified().await;
                        Ok(())
                    })
                    .await
            }));
            ready.await.unwrap();
        }
        assert!(controller
            .run(ip(5), Duration::from_secs(1), async { Ok(()) })
            .await
            .is_err());
        release.notify_waiters();
        for task in tasks {
            assert!(task.await.unwrap().is_ok());
        }
        assert!(controller.0.lock().unwrap().active.is_empty());
    }
}
