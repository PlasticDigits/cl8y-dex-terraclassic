//! Request-owned fixed-size evidence for availability rejection. Candidate
//! evaluation records clocks; only the final503 emits them, never quote successes.
use std::cell::Cell;

#[derive(Clone, Copy)]
pub(super) enum Component {
    Config,
    PoolMirror,
    HybridMirror,
    Reference,
    TaxPolicy,
    TaxActor,
    Wrapper,
    SimulationMirror,
}
impl Component {
    fn label(self) -> &'static str {
        match self {
            Self::Config => "pair_config",
            Self::PoolMirror => "pool_mirror",
            Self::HybridMirror => "hybrid_mirror",
            Self::Reference => "reference",
            Self::TaxPolicy => "tax_policy",
            Self::TaxActor => "tax_actor",
            Self::Wrapper => "wrapper",
            Self::SimulationMirror => "simulation_mirror",
        }
    }
}
#[derive(Clone, Copy)]
struct Timing {
    component: Component,
    observed_at_ms: Option<i64>,
    expires_at_ms: Option<i64>,
}
tokio::task_local! {
    static STALE: Cell<[Option<Timing>; 8]>;
}

pub(super) async fn scope<F: std::future::Future>(future: F) -> F::Output {
    STALE.scope(Cell::new([None; 8]), future).await
}

pub(super) fn record(
    component: Component,
    observed_at_ms: Option<i64>,
    expires_at_ms: Option<i64>,
) {
    // One original witness per fixed component. No per-actor/request registry or
    // allocation; work without an owning HTTP request does not emit diagnostics.
    let _ = STALE.try_with(|state| {
        let mut entries = state.get();
        entries[component as usize].get_or_insert(Timing {
            component,
            observed_at_ms,
            expires_at_ms,
        });
        state.set(entries);
    });
}

pub(super) fn emit() {
    let now = chrono::Utc::now().timestamp_millis();
    let _ = STALE.try_with(|state| {
        for timing in state.replace([None; 8]).into_iter().flatten() {
            // Clamp age magnitudes; unknown clocks remain explicit, never a
            // fabricated observation. Labels and event count are strictly fixed.
            let age_ms = timing
                .observed_at_ms
                .map(|at| now.saturating_sub(at).clamp(-86_400_000, 86_400_000));
            let expires_in_ms = timing
                .expires_at_ms
                .map(|at| at.saturating_sub(now).clamp(-86_400_000, 86_400_000));
            tracing::warn!(
                component = timing.component.label(),
                age_known = age_ms.is_some(),
                age_ms = age_ms.unwrap_or(0),
                expiry_known = expires_in_ms.is_some(),
                expires_in_ms = expires_in_ms.unwrap_or(0),
                "Local quote prerequisite stale"
            );
        }
    });
}
