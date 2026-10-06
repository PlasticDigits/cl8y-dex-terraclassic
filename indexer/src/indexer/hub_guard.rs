//! Fixed-reference containment for advisory hub reserve spots (#1391).
//! References survive outages and rejects. They never ratchet toward a manipulated
//! reserve ratio; reset is an explicit database maintenance action, not a price tick.
use super::hub_usd::HubMark;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

pub const HUB_CONFIRMATION_SECONDS: i64 = 60;
pub const HUB_MAX_DIVERGENCE_PCT: i64 = 10;
pub const HUB_POLICY: &str = "hub-fixed-reference-v1";

#[derive(Debug, Clone, FromRow)]
pub struct HubReference {
    pub ticker: String,
    pub identity_key: String,
    pub last_observed_ratio: BigDecimal,
    pub reference_ratio: BigDecimal,
    pub reference_at: DateTime<Utc>,
    pub source_pair_address: String,
    pub pending_pair_address: String,
    pub pending_since: DateTime<Utc>,
    pub last_observed_at: DateTime<Utc>,
    pub confirmed: bool,
    pub quality: String,
    pub divergence_pct: BigDecimal,
}

/// Returns an unavailable result on a replay, divergence or unconfirmed switch.
/// Ratios are measured in external USTC units so external price changes are not
/// mistaken for pool manipulation. USTR via UST1 inherits its complete deviation.
pub fn contain_mark(
    now: DateTime<Utc>,
    mark: &HubMark,
    ustc_usd: &BigDecimal,
    reference: &mut Option<HubReference>,
    identity_key: &str,
) -> bool {
    let (Some(pair), Some(as_of)) = (&mark.source_pair_address, mark.as_of) else {
        return false;
    };
    if ustc_usd <= &BigDecimal::from(0) || as_of > now {
        return false;
    }
    let ratio = (&mark.price_usd / ustc_usd).round(38);
    if ratio <= BigDecimal::from(0) {
        return false;
    }
    let state = reference.get_or_insert_with(|| HubReference {
        ticker: mark.ticker.as_str().into(),
        identity_key: identity_key.into(),
        last_observed_ratio: ratio.clone(),
        reference_ratio: ratio.clone(),
        reference_at: as_of,
        source_pair_address: pair.clone(),
        pending_pair_address: pair.clone(),
        pending_since: as_of,
        last_observed_at: as_of,
        confirmed: false,
        quality: "observing".into(),
        divergence_pct: BigDecimal::from(0),
    });
    if state.identity_key != identity_key {
        state.quality = "identity_changed".into();
        return false;
    }
    state.divergence_pct =
        ((&ratio - &state.reference_ratio) / &state.reference_ratio * BigDecimal::from(100)).abs();
    if as_of < state.last_observed_at
        || (as_of == state.last_observed_at && ratio != state.last_observed_ratio)
    {
        state.quality = "replayed".into();
        return false;
    }
    if state.divergence_pct > BigDecimal::from(HUB_MAX_DIVERGENCE_PCT) {
        state.quality = "divergent".into();
        // Invalidate a switch/bootstrap confirmation, preserving the fixed reference.
        state.pending_since = as_of;
        state.last_observed_at = as_of;
        state.last_observed_ratio = ratio;
        return false;
    }
    state.last_observed_at = as_of;
    state.last_observed_ratio = ratio;
    if pair != &state.source_pair_address || !state.confirmed {
        if pair != &state.pending_pair_address {
            state.pending_pair_address = pair.clone();
            state.pending_since = as_of;
        }
        if as_of
            .signed_duration_since(state.pending_since)
            .num_seconds()
            < HUB_CONFIRMATION_SECONDS
        {
            state.quality = "observing".into();
            return false;
        }
        state.source_pair_address = pair.clone();
        state.confirmed = true;
    }
    state.quality = "bounded_spot".into();
    true
}

#[cfg(test)]
mod tests {
    use super::super::hub_usd::HubTicker;
    use super::*;
    fn mark(now: DateTime<Utc>, value: i32, pair: &str) -> HubMark {
        HubMark {
            ticker: HubTicker::Ust1,
            asset_id: Some(1),
            price_usd: value.into(),
            source_pair_id: Some(1),
            source_pair_address: Some(pair.into()),
            tvl_usd: Some(1000.into()),
            priced_leg_depth_usd: Some(500.into()),
            as_of: Some(now),
        }
    }
    fn observe(
        at: DateTime<Utc>,
        value: i32,
        pair: &str,
        state: &mut Option<HubReference>,
    ) -> bool {
        contain_mark(
            at,
            &mark(at, value, pair),
            &1.into(),
            state,
            "canonical-identities",
        )
    }
    #[test]
    fn bootstrap_manipulation_outage_and_replay_do_not_reset_reference() {
        let now = Utc::now();
        let mut state = None;
        assert!(!observe(now, 100, "canonical", &mut state));
        let later = now + chrono::Duration::seconds(60);
        // Refreshing the same reserve timestamp cannot manufacture observation time.
        assert!(!contain_mark(
            later,
            &mark(now, 100, "canonical"),
            &1.into(),
            &mut state,
            "canonical-identities"
        ));
        assert!(observe(later, 100, "canonical", &mut state));
        let attack = later + chrono::Duration::seconds(1);
        assert!(!observe(attack, 200, "canonical", &mut state));
        assert_eq!(state.as_ref().unwrap().quality, "divergent");
        assert_eq!(
            state.as_ref().unwrap().divergence_pct,
            BigDecimal::from(100)
        );
        assert_eq!(
            state.as_ref().unwrap().reference_ratio,
            BigDecimal::from(100)
        );
        assert!(!contain_mark(
            attack,
            &mark(now, 100, "canonical"),
            &1.into(),
            &mut state,
            "canonical-identities"
        ));
        assert_eq!(state.as_ref().unwrap().quality, "replayed");
    }
    #[test]
    fn switches_require_new_observation_and_cannot_bypass_fixed_bound() {
        let now = Utc::now();
        let later = now + chrono::Duration::seconds(60);
        let mut state = None;
        observe(now, 100, "a", &mut state);
        assert!(observe(later, 100, "a", &mut state));
        let switched = later + chrono::Duration::seconds(1);
        assert!(!observe(switched, 105, "b", &mut state));
        let confirmed = switched + chrono::Duration::seconds(60);
        assert!(observe(confirmed, 105, "b", &mut state));
        assert!(!observe(
            confirmed + chrono::Duration::seconds(1),
            116,
            "b",
            &mut state
        ));
        assert_eq!(
            state.as_ref().unwrap().reference_ratio,
            BigDecimal::from(100)
        );
    }
    #[test]
    fn external_price_change_is_not_reserve_manipulation() {
        let now = Utc::now();
        let mut state = None;
        observe(now, 100, "canonical", &mut state);
        let later = now + chrono::Duration::seconds(60);
        assert!(contain_mark(
            later,
            &mark(later, 200, "canonical"),
            &2.into(),
            &mut state,
            "canonical-identities"
        ));
        assert_eq!(state.as_ref().unwrap().divergence_pct, BigDecimal::from(0));
    }
    #[test]
    fn canonical_identity_change_cannot_inherit_prior_confirmation() {
        let now = Utc::now();
        let mut state = None;
        observe(now, 100, "canonical", &mut state);
        let later = now + chrono::Duration::seconds(60);
        assert!(observe(later, 100, "canonical", &mut state));
        assert!(!contain_mark(
            later,
            &mark(later, 100, "canonical"),
            &1.into(),
            &mut state,
            "new-identity"
        ));
        assert_eq!(state.as_ref().unwrap().quality, "identity_changed");
        assert_eq!(state.as_ref().unwrap().identity_key, "canonical-identities");
    }
    #[test]
    fn same_timestamp_changed_reserve_ratio_is_replayed() {
        let now = Utc::now();
        let mut state = None;
        observe(now, 100, "canonical", &mut state);
        let later = now + chrono::Duration::seconds(60);
        assert!(observe(later, 100, "canonical", &mut state));
        assert!(!observe(later, 105, "canonical", &mut state));
        assert_eq!(state.as_ref().unwrap().quality, "replayed");
    }
}
