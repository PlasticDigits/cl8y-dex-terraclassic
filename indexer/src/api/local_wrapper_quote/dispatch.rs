//! Poll-owned native fairness. The existing global semaphore is the hard bound.
use std::sync::Mutex;
use tokio::sync::{Notify, Semaphore, SemaphorePermit, TryAcquireError};

use super::ReadSlot;

const ALL_NATIVE_READS: u8 = 0b111;

struct NativePlan {
    remaining: [u8; 2],
    registered: [u8; 2],
    borrowed: u8,
}

pub(super) struct ObservationDispatch {
    pub(super) global: Semaphore,
    pub(super) denoms: [Semaphore; 2],
    native: Mutex<Option<NativePlan>>,
    native_changed: Notify,
    #[cfg(test)]
    activity: Mutex<TestActivity>,
}

impl ObservationDispatch {
    pub(super) fn new(global: usize) -> Self {
        Self {
            global: Semaphore::new(global),
            denoms: std::array::from_fn(|_| Semaphore::new(1)),
            native: Mutex::new(None),
            native_changed: Notify::new(),
            #[cfg(test)]
            activity: Mutex::new(TestActivity::default()),
        }
    }

    /// Called once, only after authoritative bindings. No later call can renew
    /// retired bits. Common reads and no-plan helpers retain their old admission.
    pub(super) fn plan_native_reads(&self, bound: [bool; 2]) -> bool {
        let mut state = self.native.lock().unwrap();
        if state.is_some() {
            return false;
        }
        *state = Some(NativePlan {
            remaining: bound.map(|present| if present { ALL_NATIVE_READS } else { 0 }),
            registered: [0; 2],
            borrowed: 0,
        });
        true
    }

    pub(super) fn register(&self, slot: ReadSlot) -> ReadRegistration<'_> {
        let Some((denom, bit)) = native_slot(slot) else {
            return ReadRegistration::unplanned();
        };
        let mut state = self.native.lock().unwrap();
        let Some(plan) = state.as_mut() else {
            return ReadRegistration::unplanned();
        };
        if plan.remaining[denom] & bit == 0 || plan.registered[denom] & bit != 0 {
            return ReadRegistration {
                allowed: false,
                _completion: None,
            };
        }
        plan.registered[denom] |= bit;
        ReadRegistration {
            allowed: true,
            _completion: Some(ReadCompletion {
                dispatch: self,
                denom,
                bit,
                finished: false,
            }),
        }
    }

    fn peer_finished(&self, denom: usize) -> bool {
        self.native
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|plan| plan.remaining[1 - denom] == 0)
    }

    pub(super) async fn acquire_native(&self, denom: usize) -> Option<NativeAdmission<'_>> {
        loop {
            // Register before checking the monotonic mask: completion between
            // the predicate and awaiting must not lose the only notification.
            let changed = self.native_changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.peer_finished(denom) {
                match self.denoms[denom].try_acquire() {
                    Ok(permit) => {
                        return Some(NativeAdmission {
                            permit: Some(permit),
                        })
                    }
                    Err(TryAcquireError::NoPermits) => {
                        return Some(NativeAdmission { permit: None })
                    }
                    Err(TryAcquireError::Closed) => return None,
                }
            }
            tokio::select! {
                biased;
                permit = self.denoms[denom].acquire() => {
                    return permit.ok().map(|permit| NativeAdmission { permit: Some(permit) });
                }
                _ = &mut changed => {},
            }
        }
    }

    pub(super) fn record_borrow(&self) {
        if let Some(plan) = self.native.lock().unwrap().as_mut() {
            plan.borrowed = plan.borrowed.saturating_add(1).min(6);
        }
    }

    pub(super) fn borrowed_reads(&self) -> u8 {
        self.native
            .lock()
            .unwrap()
            .as_ref()
            .map_or(0, |plan| plan.borrowed)
    }

    #[cfg(test)]
    pub(super) fn observe_admission(&self, slot: ReadSlot) -> ActiveRead<'_> {
        let denom = slot.denom();
        let peer_done = denom.is_some_and(|index| self.peer_finished(index));
        let mut activity = self.activity.lock().unwrap();
        activity.active += 1;
        activity.peak = activity.peak.max(activity.active);
        if let Some(index) = denom {
            activity.native_active[index] += 1;
            activity.native_peak[index] =
                activity.native_peak[index].max(activity.native_active[index]);
            activity.parallel_before_peer_done |= activity.native_active[index] > 1 && !peer_done;
        }
        ActiveRead {
            dispatch: self,
            denom,
        }
    }

    #[cfg(test)]
    pub(super) fn activity(&self) -> TestActivity {
        *self.activity.lock().unwrap()
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Default)]
pub(super) struct TestActivity {
    active: u8,
    native_active: [u8; 2],
    pub(super) peak: u8,
    pub(super) native_peak: [u8; 2],
    pub(super) parallel_before_peer_done: bool,
}
#[cfg(test)]
pub(super) struct ActiveRead<'a> {
    dispatch: &'a ObservationDispatch,
    denom: Option<usize>,
}
#[cfg(test)]
impl Drop for ActiveRead<'_> {
    fn drop(&mut self) {
        let mut activity = self.dispatch.activity.lock().unwrap();
        activity.active -= 1;
        if let Some(index) = self.denom {
            activity.native_active[index] -= 1;
        }
    }
}

fn native_slot(slot: ReadSlot) -> Option<(usize, u8)> {
    Some(match slot {
        ReadSlot::UlunaMapping => (0, 1),
        ReadSlot::UlunaRate => (0, 2),
        ReadSlot::UlunaCap => (0, 4),
        ReadSlot::UusdMapping => (1, 1),
        ReadSlot::UusdRate => (1, 2),
        ReadSlot::UusdCap => (1, 4),
        _ => return None,
    })
}

pub(super) struct NativeAdmission<'a> {
    permit: Option<SemaphorePermit<'a>>,
}
impl NativeAdmission<'_> {
    pub(super) fn borrowed(&self) -> bool {
        self.permit.is_none()
    }
}

pub(super) struct ReadRegistration<'a> {
    allowed: bool,
    _completion: Option<ReadCompletion<'a>>,
}
impl ReadRegistration<'_> {
    fn unplanned() -> Self {
        Self {
            allowed: true,
            _completion: None,
        }
    }
    pub(super) fn allowed(&self) -> bool {
        self.allowed
    }
}

/// Non-clone ownership retires each planned slot on finish/error/drop, including
/// futures dropped before first poll. The guard outlives all read-owned permits.
struct ReadCompletion<'a> {
    dispatch: &'a ObservationDispatch,
    denom: usize,
    bit: u8,
    finished: bool,
}
impl ReadCompletion<'_> {
    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let notify = {
            let mut state = self.dispatch.native.lock().unwrap();
            let Some(plan) = state.as_mut() else {
                return;
            };
            let previous = plan.remaining[self.denom];
            plan.remaining[self.denom] &= !self.bit;
            previous != 0 && plan.remaining[self.denom] == 0
        };
        if notify {
            self.dispatch.native_changed.notify_waiters();
        }
    }
}
impl Drop for ReadCompletion<'_> {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{future::Future, task::Poll};

    async fn assert_pending<T>(future: &mut std::pin::Pin<Box<impl Future<Output = T>>>) {
        std::future::poll_fn(|cx| {
            assert!(future.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
    }

    #[tokio::test]
    async fn issue1505_borrow_waits_for_every_peer_slot_before_constructing_peer_futures() {
        for (own, peer) in [(0, 1), (1, 0)] {
            let dispatch = ObservationDispatch::new(2);
            assert!(dispatch.plan_native_reads([true, true]));
            let held = dispatch.acquire_native(own).await.unwrap();
            assert!(!held.borrowed());
            let mut pending = Box::pin(dispatch.acquire_native(own));
            assert_pending(&mut pending).await;
            let slots = if peer == 0 {
                [
                    ReadSlot::UlunaMapping,
                    ReadSlot::UlunaRate,
                    ReadSlot::UlunaCap,
                ]
            } else {
                [ReadSlot::UusdMapping, ReadSlot::UusdRate, ReadSlot::UusdCap]
            };
            let first = dispatch.register(slots[0]);
            let second = dispatch.register(slots[1]);
            let third = dispatch.register(slots[2]);
            drop(first);
            drop(second);
            assert_pending(&mut pending).await;
            // All three futures may be cancelled before first poll. Only the
            // final owned retirement can release waiting peer work.
            drop(third);
            assert!(pending.await.unwrap().borrowed());
            drop(held);
            assert_eq!(dispatch.denoms[own].available_permits(), 1);
            assert_eq!(dispatch.global.available_permits(), 2);
        }
    }

    #[tokio::test]
    async fn issue1505_borrow_duplicate_unbound_and_finished_slots_cannot_retire_or_renew_work() {
        let dispatch = ObservationDispatch::new(2);
        assert!(dispatch.plan_native_reads([true, false]));
        assert!(!dispatch.register(ReadSlot::UusdMapping).allowed());
        let mut original = dispatch.register(ReadSlot::UlunaMapping);
        let duplicate = dispatch.register(ReadSlot::UlunaMapping);
        assert!(!duplicate.allowed());
        drop(duplicate);
        assert_eq!(
            dispatch.native.lock().unwrap().as_ref().unwrap().remaining,
            [7, 0]
        );
        original._completion.as_mut().unwrap().finish();
        original._completion.as_mut().unwrap().finish();
        drop(original);
        assert_eq!(
            dispatch.native.lock().unwrap().as_ref().unwrap().remaining,
            [6, 0]
        );
        assert!(!dispatch.register(ReadSlot::UlunaMapping).allowed());
        assert!(
            !dispatch.plan_native_reads([true, true]),
            "a completed bit cannot be renewed"
        );
        assert_eq!(
            dispatch.native.lock().unwrap().as_ref().unwrap().remaining,
            [6, 0]
        );
        drop(dispatch.register(ReadSlot::UlunaRate));
        drop(dispatch.register(ReadSlot::UlunaCap));
        assert_eq!(
            dispatch.native.lock().unwrap().as_ref().unwrap().remaining,
            [0, 0]
        );
    }

    #[tokio::test]
    async fn issue1505_borrow_no_plan_retains_one_and_closed_gate_cannot_be_bypassed() {
        let dispatch = ObservationDispatch::new(2);
        let first = dispatch.acquire_native(0).await.unwrap();
        let mut second = Box::pin(dispatch.acquire_native(0));
        assert_pending(&mut second).await;
        drop(first);
        assert!(!second.await.unwrap().borrowed());
        assert!(dispatch.plan_native_reads([true, false]));
        dispatch.denoms[0].close();
        assert!(dispatch.acquire_native(0).await.is_none());
        assert_eq!(dispatch.borrowed_reads(), 0);
        assert_eq!(dispatch.global.available_permits(), 2);
    }
}
