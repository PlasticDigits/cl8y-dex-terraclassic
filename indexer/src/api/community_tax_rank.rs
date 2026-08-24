//! Tax-aware route/solve ranking (GitLab #615).
//!
//! Score / eligibility / display layer **on top of** unchanged hop LCD/DB sims.
//! Pair/router inbound stays 1:1 (**T592-1** / **H-01**). `estimated_amount_out` stays
//! pre-tax `raw_out` for execute / `min_return`.
//!
//! Invariants **R615-1–R615-8** — see `skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md`.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::api::AppState;
use crate::api::route_solver::RouteHop;
use crate::db::queries::community_tokens::{self, CommunityTokenRow};
use crate::lcd::LcdClient;

/// Columbus-5 community-tax pin. Unmigrated wasm keeps Honest router hops.
pub const COLUMBUS5_COMMUNITY_TAX_CODE_ID: i64 = 11611;

/// Catalog / TaxPreview BPS denominator (matches on-chain `BPS_DENOM`).
pub const TAX_BPS_DENOM: u128 = 10_000;

/// Cache-key fragment when neither token is catalogued and hops stay Honest.
pub const TAX_CACHE_IDENTITY_NONE: &str = "none";

#[derive(Debug, Clone, Default)]
pub struct RouterHopTaxPolicy {
    /// `COMMUNITY_TAX_OPTION2_CODE_IDS` — post-migrate / new-crate code ids.
    pub option2_code_ids: HashSet<i64>,
    /// `COMMUNITY_TAX_OPTION2_DATA_HASHES` — known option-2 wasm hashes (lowercase hex).
    pub option2_data_hashes: HashSet<String>,
}

impl RouterHopTaxPolicy {
    pub fn from_catalog(cfg: &crate::config::CommunityTaxCatalogConfig) -> Self {
        Self {
            option2_code_ids: cfg.option2_code_ids.clone(),
            option2_data_hashes: cfg.option2_data_hashes.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CataloguedTaxToken {
    pub contract_address: String,
    pub code_id: Option<i64>,
    pub data_hash: Option<String>,
    pub buy_bps: u16,
    pub sell_bps: u16,
    pub exemption_directory: bool,
}

impl CataloguedTaxToken {
    pub fn from_row(row: &CommunityTokenRow) -> Self {
        let exemption_directory = row
            .features
            .get("exemption_directory")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Self {
            contract_address: row.contract_address.trim().to_ascii_lowercase(),
            code_id: row.code_id,
            data_hash: None,
            buy_bps: row.buy_bps.unwrap_or(0).max(0) as u16,
            sell_bps: row.sell_bps.unwrap_or(0).max(0) as u16,
            exemption_directory,
        }
    }

    pub fn router_hops_tax(&self, policy: &RouterHopTaxPolicy) -> bool {
        router_hops_taxed(self.code_id, self.data_hash.as_deref(), policy)
    }
}

/// Snapshot used for cache identity + winner score + response tax fields.
#[derive(Debug, Clone, Default)]
pub struct TaxRankSnapshot {
    pub token_in: Option<CataloguedTaxToken>,
    pub token_out: Option<CataloguedTaxToken>,
    /// Lowercased address → catalog row (includes intermediates).
    pub by_addr: HashMap<String, CataloguedTaxToken>,
    pub policy: RouterHopTaxPolicy,
    /// Manager-directory exempt (`true`) or unknown/not (`false` — fail closed).
    pub trader_exempt: bool,
}

impl TaxRankSnapshot {
    pub fn cache_identity(&self) -> String {
        tax_cache_identity(self)
    }

    pub fn has_catalog_tax(&self) -> bool {
        self.token_in.is_some() || self.token_out.is_some() || !self.by_addr.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxRankResult {
    pub raw_out: u128,
    pub net_out: u128,
    pub eligible: bool,
    pub buy_tax_bps: u16,
    pub sell_tax_bps: u16,
    pub tax_kind: &'static str,
    pub tax_notes: String,
    pub router_hops_tax: bool,
}

/// Live **11611** stays Honest until an explicit option-2 code id / hash flip.
/// Do not treat unmigrated 11611 as option 2.
pub fn router_hops_taxed(
    code_id: Option<i64>,
    data_hash: Option<&str>,
    policy: &RouterHopTaxPolicy,
) -> bool {
    if let Some(hash) = data_hash.map(normalize_hash).filter(|h| !h.is_empty()) {
        if policy.option2_data_hashes.contains(&hash) {
            return true;
        }
    }
    if let Some(id) = code_id {
        if policy.option2_code_ids.contains(&id) {
            return true;
        }
        if id == COLUMBUS5_COMMUNITY_TAX_CODE_ID {
            return false;
        }
    }
    false
}

fn normalize_hash(raw: &str) -> String {
    raw.trim().trim_start_matches("0x").to_ascii_lowercase()
}

/// `net_out = raw_out - floor(raw_out * buy_bps / 10000)` when buy tax applies.
pub fn buy_split_net(raw_out: u128, buy_bps: u16) -> u128 {
    if buy_bps == 0 || raw_out == 0 {
        return raw_out;
    }
    let tax = raw_out.saturating_mul(u128::from(buy_bps)) / TAX_BPS_DENOM;
    raw_out.saturating_sub(tax)
}

/// Buy split applies for pair-direct (1 hop) always, and for multi-hop only when
/// that token's wasm taxes router hops (option 2). Honest 11611 hops stay raw.
pub fn buy_tax_applies(token_out_taxed: bool, hop_count: usize, router_hops_tax: bool) -> bool {
    if !token_out_taxed {
        return false;
    }
    hop_count <= 1 || router_hops_tax
}

/// Drop paths that **sell** a catalogued tax token as a middle hop (not `token_in`)
/// when that token's wasm taxes official-router hops. Fail-closed only for option 2.
pub fn path_sells_middle_tax_hop(
    hops: &[RouteHop],
    token_in: &str,
    snapshot: &TaxRankSnapshot,
) -> bool {
    let tin = token_in.trim().to_ascii_lowercase();
    for hop in hops {
        let offer = hop.offer_token.trim().to_ascii_lowercase();
        if offer == tin {
            continue;
        }
        let Some(row) = snapshot.by_addr.get(&offer) else {
            continue;
        };
        if row.router_hops_tax(&snapshot.policy) {
            return true;
        }
    }
    false
}

pub fn score_path(
    raw_out: u128,
    hops: &[RouteHop],
    token_in: &str,
    snapshot: &TaxRankSnapshot,
) -> TaxRankResult {
    let hop_count = hops.len();
    let eligible = !path_sells_middle_tax_hop(hops, token_in, snapshot);

    let out_row = snapshot.token_out.as_ref();
    let in_row = snapshot.token_in.as_ref();
    let out_rh = out_row
        .map(|r| r.router_hops_tax(&snapshot.policy))
        .unwrap_or(false);
    let in_rh = in_row
        .map(|r| r.router_hops_tax(&snapshot.policy))
        .unwrap_or(false);
    let router_hops_tax = out_rh || in_rh;

    let buy_catalog = out_row.map(|r| r.buy_bps).unwrap_or(0);
    let sell_catalog = in_row.map(|r| r.sell_bps).unwrap_or(0);
    let buy_tax_bps = if snapshot.trader_exempt {
        0
    } else {
        buy_catalog
    };
    let sell_tax_bps = if snapshot.trader_exempt {
        0
    } else {
        sell_catalog
    };

    let apply_buy = buy_tax_applies(out_row.is_some() && buy_tax_bps > 0, hop_count, out_rh);
    let net_out = if apply_buy {
        buy_split_net(raw_out, buy_tax_bps)
    } else {
        raw_out
    };

    let (tax_kind, tax_notes) = tax_kind_and_notes(
        out_row.is_some(),
        in_row.is_some(),
        apply_buy,
        snapshot.trader_exempt,
        out_rh,
        eligible,
    );

    TaxRankResult {
        raw_out,
        net_out,
        eligible,
        buy_tax_bps: if apply_buy { buy_tax_bps } else { 0 },
        sell_tax_bps,
        tax_kind,
        tax_notes,
        router_hops_tax,
    }
}

fn tax_kind_and_notes(
    token_out_tax: bool,
    token_in_tax: bool,
    apply_buy: bool,
    exempt: bool,
    out_router_hops_tax: bool,
    eligible: bool,
) -> (&'static str, String) {
    if !eligible {
        return (
            "skipped_middle_sell",
            "Path sells a catalogued tax token as a middle hop under option-2 wasm; skipped (GitLab #615).".into(),
        );
    }
    if exempt && (token_out_tax || token_in_tax) {
        return (
            "exempt",
            "Manager-directory exempt trader: 0 bps (GitLab #609 / #615).".into(),
        );
    }
    if apply_buy {
        return (
            "buy",
            "token_out buy split applied to net rank / You Receive; estimated_amount_out stays pre-tax (H-01).".into(),
        );
    }
    if token_out_tax && !out_router_hops_tax {
        return (
            "honest_hops",
            "Unmigrated 11611 / Honest hops: multi-hop buy tax not applied to net (T592-13 / #615).".into(),
        );
    }
    if token_in_tax {
        return (
            "sell_in",
            "token_in extra-debit is the same on every remaining path; rank uses ask raw/net only."
                .into(),
        );
    }
    (
        "none",
        "No catalog buy/sell adjustment for this snapshot.".into(),
    )
}

pub fn tax_cache_identity(snapshot: &TaxRankSnapshot) -> String {
    if !snapshot.has_catalog_tax() && !snapshot.policy_has_option2() {
        return TAX_CACHE_IDENTITY_NONE.to_string();
    }
    let in_buy = snapshot.token_in.as_ref().map(|r| r.buy_bps).unwrap_or(0);
    let in_sell = snapshot.token_in.as_ref().map(|r| r.sell_bps).unwrap_or(0);
    let out_buy = snapshot.token_out.as_ref().map(|r| r.buy_bps).unwrap_or(0);
    let out_sell = snapshot.token_out.as_ref().map(|r| r.sell_bps).unwrap_or(0);
    let rh = snapshot
        .token_in
        .as_ref()
        .map(|r| r.router_hops_tax(&snapshot.policy))
        .unwrap_or(false)
        || snapshot
            .token_out
            .as_ref()
            .map(|r| r.router_hops_tax(&snapshot.policy))
            .unwrap_or(false);
    format!(
        "b{}s{}/b{}s{}/rh{}/e{}",
        in_buy,
        in_sell,
        out_buy,
        out_sell,
        u8::from(rh),
        u8::from(snapshot.trader_exempt)
    )
}

impl RouterHopTaxPolicy {
    fn is_empty(&self) -> bool {
        self.option2_code_ids.is_empty() && self.option2_data_hashes.is_empty()
    }
}

impl TaxRankSnapshot {
    fn policy_has_option2(&self) -> bool {
        !self.policy.is_empty()
    }
}

pub async fn load_tax_rank_snapshot(
    state: &AppState,
    token_in: &str,
    token_out: &str,
    extra_addrs: &[String],
    trader: Option<&str>,
) -> TaxRankSnapshot {
    let policy = RouterHopTaxPolicy::from_catalog(&state.community_tax);
    let mut addrs = Vec::with_capacity(2 + extra_addrs.len());
    addrs.push(token_in.trim().to_ascii_lowercase());
    addrs.push(token_out.trim().to_ascii_lowercase());
    for a in extra_addrs {
        let l = a.trim().to_ascii_lowercase();
        if !l.is_empty() {
            addrs.push(l);
        }
    }
    addrs.sort();
    addrs.dedup();

    let rows = match community_tokens::get_by_addresses(&state.pool, &addrs).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "community tax catalog lookup failed; ranking without tax layer");
            return TaxRankSnapshot {
                policy,
                ..Default::default()
            };
        }
    };

    let mut by_addr = HashMap::new();
    for row in &rows {
        let tok = CataloguedTaxToken::from_row(row);
        by_addr.insert(tok.contract_address.clone(), tok);
    }
    let tin = token_in.trim().to_ascii_lowercase();
    let tout = token_out.trim().to_ascii_lowercase();
    let token_in_row = by_addr.get(&tin).cloned();
    let token_out_row = by_addr.get(&tout).cloned();

    let mut trader_exempt = false;
    if let Some(addr) = trader.map(str::trim).filter(|s| !s.is_empty()) {
        trader_exempt =
            resolve_trader_exempt(&state.lcd, &token_in_row, &token_out_row, addr).await;
    }

    TaxRankSnapshot {
        token_in: token_in_row,
        token_out: token_out_row,
        by_addr,
        policy,
        trader_exempt,
    }
}

#[derive(Debug, Deserialize)]
struct IsExemptLcd {
    #[serde(default)]
    manager: bool,
    #[serde(default)]
    protocol: bool,
}

/// LCD `IsProtocolExempt` also returns the manager-directory flag (#609).
/// Unknown / LCD failure → fail closed (keep catalog bps).
async fn resolve_trader_exempt(
    lcd: &LcdClient,
    token_in: &Option<CataloguedTaxToken>,
    token_out: &Option<CataloguedTaxToken>,
    trader: &str,
) -> bool {
    for tok in [token_in.as_ref(), token_out.as_ref()]
        .into_iter()
        .flatten()
    {
        if !tok.exemption_directory {
            continue;
        }
        let q = serde_json::json!({ "is_protocol_exempt": { "address": trader } });
        match lcd
            .query_contract::<IsExemptLcd>(&tok.contract_address, &q)
            .await
        {
            Ok(v) if v.manager || v.protocol => return true,
            Ok(_) => {}
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    token = %tok.contract_address,
                    "tax exempt LCD failed; fail closed (keep bps)"
                );
            }
        }
    }
    false
}

pub fn extra_addrs_from_hops(hops: &[RouteHop]) -> Vec<String> {
    hops.iter()
        .flat_map(|h| [h.offer_token.clone(), h.ask_token.clone()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hop(offer: &str, ask: &str) -> RouteHop {
        RouteHop {
            pair: "terra1pair".into(),
            offer_token: offer.into(),
            ask_token: ask.into(),
        }
    }

    fn tax_tok(addr: &str, code_id: i64, buy: u16, sell: u16) -> CataloguedTaxToken {
        CataloguedTaxToken {
            contract_address: addr.to_ascii_lowercase(),
            code_id: Some(code_id),
            data_hash: None,
            buy_bps: buy,
            sell_bps: sell,
            exemption_directory: true,
        }
    }

    fn snap(
        token_in: Option<CataloguedTaxToken>,
        token_out: Option<CataloguedTaxToken>,
        policy: RouterHopTaxPolicy,
        exempt: bool,
    ) -> TaxRankSnapshot {
        let mut by_addr = HashMap::new();
        if let Some(ref t) = token_in {
            by_addr.insert(t.contract_address.clone(), t.clone());
        }
        if let Some(ref t) = token_out {
            by_addr.insert(t.contract_address.clone(), t.clone());
        }
        TaxRankSnapshot {
            token_in,
            token_out,
            by_addr,
            policy,
            trader_exempt: exempt,
        }
    }

    const TAX: &str = "terra1tax0000000000000000000000000000000001";
    const UST1: &str = "terra1ust1000000000000000000000000000000001";
    const USTR: &str = "terra1ustr000000000000000000000000000000001";

    #[test]
    fn buy_split_matches_tax_preview_floor() {
        assert_eq!(buy_split_net(1_000_000, 100), 990_000);
        assert_eq!(buy_split_net(10_000, 1), 9_999);
        assert_eq!(buy_split_net(1, 100), 1, "floor(1*100/10000)=0");
        assert_eq!(buy_split_net(50, 0), 50);
    }

    #[test]
    fn tax_in_paths_both_eligible_rank_max_raw() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 200);
        let s = snap(Some(t), None, RouterHopTaxPolicy::default(), false);
        let direct = vec![hop(TAX, UST1)];
        let via = vec![hop(TAX, USTR), hop(USTR, UST1)];
        let a = score_path(100, &direct, TAX, &s);
        let b = score_path(90, &via, TAX, &s);
        assert!(a.eligible && b.eligible);
        assert_eq!(a.net_out, 100);
        assert_eq!(b.net_out, 90);
        assert!(a.net_out > b.net_out);
        assert_eq!(a.sell_tax_bps, 200);
    }

    #[test]
    fn token_out_tax_pair_direct_net_lt_raw() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 0);
        let s = snap(None, Some(t), RouterHopTaxPolicy::default(), false);
        let hops = vec![hop(UST1, TAX)];
        let r = score_path(1_000_000, &hops, UST1, &s);
        assert!(r.eligible);
        assert_eq!(r.raw_out, 1_000_000);
        assert_eq!(r.net_out, 990_000);
        assert_eq!(r.buy_tax_bps, 100);
        assert_eq!(r.tax_kind, "buy");
    }

    #[test]
    fn option2_skips_middle_tax_hop() {
        let t = tax_tok(TAX, 99, 100, 200);
        let mut policy = RouterHopTaxPolicy::default();
        policy.option2_code_ids.insert(99);
        let mut s = snap(None, None, policy, false);
        s.by_addr.insert(TAX.to_string(), t);
        let via_tax = vec![hop(UST1, TAX), hop(TAX, USTR)];
        let direct = vec![hop(UST1, USTR)];
        let skipped = score_path(9_000_000, &via_tax, UST1, &s);
        let kept = score_path(1_000_000, &direct, UST1, &s);
        assert!(!skipped.eligible);
        assert!(kept.eligible);
        assert_eq!(kept.net_out, 1_000_000);
    }

    #[test]
    fn unmigrated_11611_middle_hop_stays_eligible() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 200);
        let mut s = snap(None, None, RouterHopTaxPolicy::default(), false);
        s.by_addr.insert(TAX.to_string(), t);
        let via_tax = vec![hop(UST1, TAX), hop(TAX, USTR)];
        let r = score_path(5_000_000, &via_tax, UST1, &s);
        assert!(r.eligible, "11611 Honest hops must not skip middle TAX");
        assert_eq!(r.net_out, 5_000_000);
        assert_eq!(r.tax_kind, "none");
    }

    #[test]
    fn exempt_trader_net_equals_raw() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 250, 250);
        let s = snap(None, Some(t), RouterHopTaxPolicy::default(), true);
        let hops = vec![hop(UST1, TAX)];
        let r = score_path(1_000_000, &hops, UST1, &s);
        assert_eq!(r.net_out, r.raw_out);
        assert_eq!(r.buy_tax_bps, 0);
        assert_eq!(r.tax_kind, "exempt");
    }

    #[test]
    fn unknown_exempt_fail_closed_keeps_bps() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 250, 0);
        let s = snap(None, Some(t), RouterHopTaxPolicy::default(), false);
        let hops = vec![hop(UST1, TAX)];
        let r = score_path(10_000, &hops, UST1, &s);
        assert_eq!(r.net_out, 9_750);
        assert_eq!(r.buy_tax_bps, 250);
    }

    #[test]
    fn honest_multihop_buy_does_not_shrink_net() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 0);
        let s = snap(None, Some(t), RouterHopTaxPolicy::default(), false);
        let hops = vec![hop(UST1, USTR), hop(USTR, TAX)];
        let r = score_path(1_000_000, &hops, UST1, &s);
        assert_eq!(r.net_out, 1_000_000);
        assert_eq!(r.tax_kind, "honest_hops");
    }

    #[test]
    fn option2_multihop_buy_shrinks_net() {
        let t = tax_tok(TAX, 42, 100, 0);
        let mut policy = RouterHopTaxPolicy::default();
        policy.option2_code_ids.insert(42);
        let s = snap(None, Some(t), policy, false);
        let hops = vec![hop(UST1, USTR), hop(USTR, TAX)];
        let r = score_path(1_000_000, &hops, UST1, &s);
        assert_eq!(r.net_out, 990_000);
        assert_eq!(r.tax_kind, "buy");
    }

    #[test]
    fn inbound_unchanged_score_does_not_resize_offer() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 200);
        let s = snap(Some(t), None, RouterHopTaxPolicy::default(), false);
        let hops = vec![hop(TAX, UST1)];
        let r = score_path(777, &hops, TAX, &s);
        assert_eq!(r.raw_out, 777);
        assert_eq!(r.net_out, 777);
    }

    #[test]
    fn cache_identity_isolates_tax_vs_ordinary() {
        let ordinary = TaxRankSnapshot::default();
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 0);
        let taxed = snap(None, Some(t), RouterHopTaxPolicy::default(), false);
        assert_eq!(ordinary.cache_identity(), TAX_CACHE_IDENTITY_NONE);
        assert_ne!(taxed.cache_identity(), ordinary.cache_identity());
        let exempt = snap(
            None,
            Some(tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 0)),
            RouterHopTaxPolicy::default(),
            true,
        );
        assert_ne!(taxed.cache_identity(), exempt.cache_identity());
    }

    #[test]
    fn cache_identity_isolates_option2_flag() {
        let t = tax_tok(TAX, 7, 0, 0);
        let honest = snap(None, Some(t.clone()), RouterHopTaxPolicy::default(), false);
        let mut policy = RouterHopTaxPolicy::default();
        policy.option2_code_ids.insert(7);
        let opt2 = snap(None, Some(t), policy, false);
        assert_ne!(honest.cache_identity(), opt2.cache_identity());
    }

    #[test]
    fn router_hops_taxed_11611_pin() {
        let empty = RouterHopTaxPolicy::default();
        assert!(!router_hops_taxed(
            Some(COLUMBUS5_COMMUNITY_TAX_CODE_ID),
            None,
            &empty
        ));
        let mut hashed = RouterHopTaxPolicy::default();
        hashed.option2_data_hashes.insert("abc123def".into());
        assert!(router_hops_taxed(
            Some(COLUMBUS5_COMMUNITY_TAX_CODE_ID),
            Some("0xABC123DEF"),
            &hashed
        ));
        let mut codes = RouterHopTaxPolicy::default();
        codes.option2_code_ids.insert(99);
        assert!(router_hops_taxed(Some(99), None, &codes));
        assert!(!router_hops_taxed(
            Some(COLUMBUS5_COMMUNITY_TAX_CODE_ID),
            None,
            &codes
        ));
    }

    #[test]
    fn first_seen_tie_break_is_callers_job() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 0, 0);
        let s = snap(Some(t), None, RouterHopTaxPolicy::default(), false);
        let a = score_path(50, &[hop(TAX, UST1)], TAX, &s);
        let b = score_path(50, &[hop(TAX, USTR), hop(USTR, UST1)], TAX, &s);
        assert_eq!(a.net_out, b.net_out);
    }
}
