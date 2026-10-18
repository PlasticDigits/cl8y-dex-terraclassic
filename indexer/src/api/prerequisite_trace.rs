//! Request-owned fixed-size evidence for availability rejection. Candidate
//! evaluation records clocks and fixed missing stages; only the final503 emits
//! them, never quote successes. Missing stages never invent observation clocks.
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
/// A candidate-stage witness, not a claim that this was the sole failed path.
/// Stages share the existing eight component slots; no dynamic registry.
#[derive(Clone, Copy)]
pub(super) enum MissingStage {
    AssetIdentity,
    MirrorAdmission,
    PairAdmission,
    TaxPair,
    TaxActor,
    TaxBalance,
    ReferencePrice,
    SimulationMirror,
    SimulationFee,
    NativeMapping,
    NativeRouter,
    NativeTaxRate,
    NativeTaxCap,
}
impl MissingStage {
    #[cfg(test)]
    const ALL: [Self; 13] = [
        Self::AssetIdentity,
        Self::MirrorAdmission,
        Self::PairAdmission,
        Self::TaxPair,
        Self::TaxActor,
        Self::TaxBalance,
        Self::ReferencePrice,
        Self::SimulationMirror,
        Self::SimulationFee,
        Self::NativeMapping,
        Self::NativeRouter,
        Self::NativeTaxRate,
        Self::NativeTaxCap,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::AssetIdentity => "asset_identity",
            Self::MirrorAdmission => "mirror_admission",
            Self::PairAdmission => "pair_admission",
            Self::TaxPair => "tax_pair",
            Self::TaxActor => "tax_actor",
            Self::TaxBalance => "tax_balance",
            Self::ReferencePrice => "reference_price",
            Self::SimulationMirror => "simulation_mirror",
            Self::SimulationFee => "simulation_fee",
            Self::NativeMapping => "native_mapping",
            Self::NativeRouter => "native_router",
            Self::NativeTaxRate => "native_tax_rate",
            Self::NativeTaxCap => "native_tax_cap",
        }
    }
    fn component(self) -> Component {
        match self {
            Self::AssetIdentity | Self::PairAdmission => Component::Config,
            Self::MirrorAdmission | Self::SimulationMirror | Self::SimulationFee => {
                Component::SimulationMirror
            }
            Self::TaxPair => Component::TaxPolicy,
            Self::TaxActor | Self::TaxBalance => Component::TaxActor,
            Self::ReferencePrice => Component::Reference,
            Self::NativeMapping | Self::NativeRouter | Self::NativeTaxRate | Self::NativeTaxCap => {
                Component::Wrapper
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Timing {
    component: Component,
    observed_at_ms: Option<i64>,
    expires_at_ms: Option<i64>,
    missing_stage: Option<MissingStage>,
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
            missing_stage: None,
        });
        state.set(entries);
    });
}

pub(super) fn record_missing(stage: MissingStage) {
    // The same eight component slots retain the first original witness, whether
    // missing or stale. No request owner means nothing is retained or emitted.
    let _ = STALE.try_with(|state| {
        let mut entries = state.get();
        let component = stage.component();
        entries[component as usize].get_or_insert(Timing {
            component,
            observed_at_ms: None,
            expires_at_ms: None,
            missing_stage: Some(stage),
        });
        state.set(entries);
    });
}

pub(super) fn emit() {
    let now = chrono::Utc::now().timestamp_millis();
    let _ = STALE.try_with(|state| {
        for timing in state.replace([None; 8]).into_iter().flatten() {
            if let Some(stage) = timing.missing_stage {
                tracing::warn!(
                    component = timing.component.label(),
                    stage = stage.label(),
                    age_known = false,
                    age_ms = 0,
                    expiry_known = false,
                    expires_in_ms = 0,
                    "Local quote prerequisite missing"
                );
                continue;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_missing_witness_registry_keeps_eight_original_component_slots() {
        // Outside an HTTP-owned scope, nothing is stored globally.
        record_missing(MissingStage::ReferencePrice);
        scope(async {
            for _ in 0..64 {
                for stage in MissingStage::ALL {
                    record_missing(stage);
                }
            }
            record(Component::Config, Some(100), Some(200));
            record(Component::PoolMirror, Some(101), Some(201));
            record(Component::HybridMirror, Some(102), Some(202));
            STALE.with(|state| {
                let entries = state.get();
                assert_eq!(entries.iter().flatten().count(), 8);
                let config = entries[Component::Config as usize].unwrap();
                assert!(matches!(
                    config.missing_stage,
                    Some(MissingStage::AssetIdentity)
                ));
                assert_eq!(config.observed_at_ms, None);
                assert_eq!(config.expires_at_ms, None);
            });
            emit();
            emit(); // Cleared evidence cannot double-log inside the same request.
        })
        .await;
        logs_assert(|lines: &[&str]| {
            let total = lines
                .iter()
                .filter(|line| line.contains("Local quote prerequisite"))
                .count();
            if total == 8 {
                Ok(())
            } else {
                Err(format!("expected eight total mixed witnesses, got {total}"))
            }
        });
        scope(async {
            STALE.with(|state| assert!(state.get().iter().all(Option::is_none)));
            record(Component::Reference, Some(10), Some(20));
            record_missing(MissingStage::ReferencePrice);
            STALE.with(|state| {
                let first = state.get()[Component::Reference as usize].unwrap();
                assert!(first.missing_stage.is_none());
                assert_eq!(first.observed_at_ms, Some(10));
                assert_eq!(first.expires_at_ms, Some(20));
            });
        })
        .await;
    }
}
