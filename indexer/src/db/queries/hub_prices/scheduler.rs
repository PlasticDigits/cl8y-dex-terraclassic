//! Independently owned SQL refresh scheduling; requests never wake these jobs.
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::Notify;

pub(super) async fn run<F, Work>(
    interval: Duration,
    pool_completed: Option<Arc<Notify>>,
    mut refresh: F,
) where
    F: FnMut() -> Work,
    Work: Future<Output = ()>,
{
    let mut timer =
        tokio::time::interval(interval.clamp(Duration::from_secs(1), Duration::from_secs(5)));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if let Some(signal) = pool_completed.as_ref() {
            tokio::select! {
                biased;
                _ = signal.notified() => {
                    // A completed raw cycle consumes the next scheduled turn;
                    // coalesce simultaneous timer/wake work instead of replaying it.
                    timer.reset();
                    tracing::info!(phase = "reference_schedule", trigger = "pool_completion",
                        "Reference refresh scheduled");
                },
                _ = timer.tick() => {},
            }
        } else {
            timer.tick().await;
        }
        refresh().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ReferencePrices;
    use chrono::{Duration as ChronoDuration, Utc};
    use std::sync::{
        atomic::{AtomicI64, Ordering},
        Mutex,
    };

    #[tokio::test(start_paused = true)]
    async fn issue1505_pool_wakes_coalesce_while_owned_mark_pass_is_blocked() {
        use std::sync::atomic::AtomicUsize;
        let calls = Arc::new(AtomicUsize::new(0));
        let signal = Arc::new(Notify::new());
        let first_started = Arc::new(Notify::new());
        let second_started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let work_calls = calls.clone();
        let work_first = first_started.clone();
        let work_second = second_started.clone();
        let work_release = release.clone();
        let worker = tokio::spawn(run(
            Duration::from_secs(5),
            Some(signal.clone()),
            move || {
                let calls = work_calls.clone();
                let first = work_first.clone();
                let second = work_second.clone();
                let release = work_release.clone();
                async move {
                    match calls.fetch_add(1, Ordering::SeqCst) {
                        0 => {
                            first.notify_one();
                            release.notified().await;
                        }
                        1 => second.notify_one(),
                        _ => {}
                    }
                }
            },
        ));
        first_started.notified().await;
        for _ in 0..100 {
            signal.notify_one();
        }
        tokio::time::sleep(Duration::from_secs(6)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1, "one mark pass at a time");
        release.notify_one();
        second_started.notified().await;
        tokio::task::yield_now().await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "one coalesced wake consumes the overdue timer turn"
        );
        worker.abort();
        let _ = worker.await;
    }

    #[tokio::test(start_paused = true)]
    async fn issue1505_completed_pool_wakes_atomic_reference_before_original_expiry() {
        let original =
            chrono::DateTime::from_timestamp_millis(Utc::now().timestamp_millis()).unwrap();
        let raw = Arc::new(AtomicI64::new(original.timestamp_millis()));
        let mut initial = ReferencePrices::synthetic([1, 2].into_iter());
        for price in initial.prices.values_mut() {
            price.as_of = original;
        }
        let marks = Arc::new(Mutex::new(initial.clone()));
        let published = Arc::new(Mutex::new(initial));
        let signal = Arc::new(Notify::new());
        let mark_raw = raw.clone();
        let mark_state = marks.clone();
        let producer = tokio::spawn(run(
            Duration::from_secs(5),
            Some(signal.clone()),
            move || {
                let raw = mark_raw.clone();
                let state = mark_state.clone();
                async move {
                    // The atomic producer captures reserves before its SQL commit.
                    let as_of = chrono::DateTime::from_timestamp_millis(raw.load(Ordering::SeqCst))
                        .unwrap();
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let mut next = state.lock().unwrap().clone();
                    for price in next.prices.values_mut() {
                        price.as_of = as_of;
                    }
                    *state.lock().unwrap() = next;
                }
            },
        ));
        let consumer_marks = marks.clone();
        let consumer_published = published.clone();
        let consumer = tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(2));
            loop {
                timer.tick().await;
                *consumer_published.lock().unwrap() = consumer_marks.lock().unwrap().clone();
            }
        });
        // A10s pool cycle's selected source completes5.6s later, just after the
        // independent5s mark tick captured the old original reserve generation.
        tokio::time::sleep(Duration::from_millis(15_600)).await;
        let replacement = original + ChronoDuration::seconds(10);
        raw.store(replacement.timestamp_millis(), Ordering::SeqCst);
        signal.notify_one();
        tokio::time::sleep(Duration::from_millis(4_500)).await;
        let snapshot = published.lock().unwrap().clone();
        producer.abort();
        consumer.abort();
        let _ = producer.await;
        let _ = consumer.await;
        assert!(snapshot.fair_output(1, 2, 10_000, original + ChronoDuration::milliseconds(20_100)).is_some(),
            "initialized consumers must receive the completed original replacement before old20s expiry");
        assert!(
            snapshot
                .prices
                .values()
                .all(|price| price.as_of == replacement),
            "one whole atomic cohort keeps the original pool time, never the notification time"
        );
        assert!(snapshot
            .fair_output(
                1,
                2,
                10_000,
                replacement + ChronoDuration::milliseconds(20_001)
            )
            .is_none());
    }
}
