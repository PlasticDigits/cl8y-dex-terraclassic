//! Indexing readiness is distinct from cheap API liveness and quote eligibility (#1464).
use std::sync::{Arc, Mutex};

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IndexerStatus {
    Starting,
    Running,
    ProviderUnavailable,
    Halted,
    Stopped,
}

/// Fixed reasons only: upstream bodies, endpoint URLs and parser details stay private.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct IndexerHealthSnapshot {
    pub status: IndexerStatus,
    pub ready: bool,
    pub checkpoint_height: i64,
    pub latest_height: Option<i64>,
    pub pending_height: Option<i64>,
    pub reason: Option<&'static str>,
}

#[derive(Clone)]
pub struct IndexerHealth {
    state: Arc<Mutex<IndexerHealthSnapshot>>,
    halted: tokio_util::sync::CancellationToken,
}

impl Default for IndexerHealth {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(IndexerHealthSnapshot {
                status: IndexerStatus::Starting,
                ready: false,
                checkpoint_height: 0,
                latest_height: None,
                pending_height: None,
                reason: None,
            })),
            halted: tokio_util::sync::CancellationToken::new(),
        }
    }
}

impl IndexerHealth {
    /// Cancellation notification for independently scheduled quote producers.
    pub fn halt_token(&self) -> tokio_util::sync::CancellationToken {
        self.halted.clone()
    }

    pub fn snapshot(&self) -> IndexerHealthSnapshot {
        self.state.lock().unwrap().clone()
    }

    pub fn progress(&self, checkpoint: i64, latest: i64) {
        let mut snapshot = self.state.lock().unwrap();
        if snapshot.status == IndexerStatus::Halted {
            return;
        }
        snapshot.status = IndexerStatus::Running;
        snapshot.checkpoint_height = checkpoint;
        snapshot.latest_height = Some(latest);
        snapshot.pending_height = (checkpoint < latest).then_some(checkpoint.saturating_add(1));
        snapshot.ready = checkpoint >= latest;
        snapshot.reason = None;
    }

    pub fn unavailable(&self, pending: Option<i64>) {
        let mut snapshot = self.state.lock().unwrap();
        if snapshot.status == IndexerStatus::Halted {
            return;
        }
        snapshot.status = IndexerStatus::ProviderUnavailable;
        snapshot.ready = false;
        snapshot.pending_height = pending;
        snapshot.reason = Some("provider_unavailable");
    }

    pub fn halt(&self, reason: &'static str) {
        let mut snapshot = self.state.lock().unwrap();
        snapshot.status = IndexerStatus::Halted;
        snapshot.ready = false;
        snapshot.reason = Some(reason);
        self.halted.cancel();
    }

    pub fn stopped(&self) {
        let mut snapshot = self.state.lock().unwrap();
        if snapshot.status != IndexerStatus::Halted {
            snapshot.status = IndexerStatus::Stopped;
            snapshot.ready = false;
            snapshot.reason = Some("stopped");
        }
    }
}

/// Keep API serving after indexing completes/fails. Shutdown owns and joins both tasks;
/// an ended or panicked indexer latches an unready state instead of resetting quote caches.
pub async fn supervise_tasks<F>(
    mut indexer: tokio::task::JoinHandle<()>,
    mut api: tokio::task::JoinHandle<()>,
    health: IndexerHealth,
    cancel: tokio_util::sync::CancellationToken,
    shutdown: F,
) where
    F: std::future::Future<Output = ()>,
{
    tokio::pin!(shutdown);
    let mut indexer_ended = false;
    let mut api_ended = false;
    loop {
        tokio::select! {
            result = &mut indexer, if !indexer_ended => {
                indexer_ended = true;
                if result.is_err() {
                    health.halt("indexer_task_failure");
                } else if health.snapshot().status != IndexerStatus::Halted {
                    health.halt("indexer_task_ended");
                }
                tracing::error!("Indexer halted; API remains available with unready indexing status");
            },
            _ = &mut api => {
                api_ended = true;
                tracing::warn!("API task ended");
                break;
            },
            _ = &mut shutdown => {
                tracing::info!("Shutdown signal received");
                break;
            }
        }
    }
    cancel.cancel();
    if !indexer_ended {
        join_or_abort(indexer).await;
    }
    if !api_ended {
        join_or_abort(api).await;
    }
}

async fn join_or_abort(mut task: tokio::task::JoinHandle<()>) {
    if tokio::time::timeout(std::time::Duration::from_secs(5), &mut task)
        .await
        .is_err()
    {
        task.abort();
        let _ = task.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recovery1464_fatal_indexer_keeps_api_alive_until_shutdown() {
        let health = IndexerHealth::default();
        health.progress(40, 41);
        let indexer_health = health.clone();
        let indexer = tokio::spawn(async move {
            indexer_health.halt("reorg");
        });
        let cancel = tokio_util::sync::CancellationToken::new();
        let api_cancel = cancel.clone();
        let (api_started, api_running) = tokio::sync::oneshot::channel();
        let api_stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = api_stopped.clone();
        let api = tokio::spawn(async move {
            api_started.send(()).unwrap();
            api_cancel.cancelled().await;
            stopped.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(supervise_tasks(
            indexer,
            api,
            health.clone(),
            cancel.clone(),
            async move {
                let _ = shutdown.await;
            },
        ));
        api_running.await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(health.snapshot().status, IndexerStatus::Halted);
        assert_eq!(health.snapshot().reason, Some("reorg"));
        assert!(!task.is_finished());
        assert!(!cancel.is_cancelled());
        assert!(!api_stopped.load(std::sync::atomic::Ordering::SeqCst));
        stop.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();
        assert!(api_stopped.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn recovery1464_panicked_indexer_is_unready_without_stopping_api() {
        let health = IndexerHealth::default();
        let cancel = tokio_util::sync::CancellationToken::new();
        let api_cancel = cancel.clone();
        let indexer = tokio::spawn(async { panic!("synthetic indexing failure") });
        let api = tokio::spawn(async move {
            api_cancel.cancelled().await;
        });
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(supervise_tasks(
            indexer,
            api,
            health.clone(),
            cancel.clone(),
            async move {
                let _ = shutdown.await;
            },
        ));
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(health.snapshot().reason, Some("indexer_task_failure"));
        assert!(!health.snapshot().ready);
        assert!(!task.is_finished());
        stop.send(()).unwrap();
        task.await.unwrap();
    }

    #[test]
    fn recovery1464_availability_recovers_but_integrity_halt_is_latched() {
        let health = IndexerHealth::default();
        assert!(!health.snapshot().ready);
        health.progress(40, 41);
        health.unavailable(Some(41));
        assert_eq!(health.snapshot().checkpoint_height, 40);
        assert_eq!(health.snapshot().pending_height, Some(41));
        assert!(!health.snapshot().ready);
        health.progress(41, 41);
        assert!(health.snapshot().ready);
        health.halt("reorg");
        health.progress(42, 42);
        health.unavailable(Some(43));
        health.stopped();
        assert_eq!(health.snapshot().status, IndexerStatus::Halted);
        assert_eq!(health.snapshot().checkpoint_height, 41);
        assert_eq!(health.snapshot().reason, Some("reorg"));
        assert!(!health.snapshot().ready);
    }
}
