//! Route-local trading tax discovery, funded sizing and net ranking (#1381).
//! Upgraded hop simulations already include taxes; scoring never applies a second haircut.
//! Pair economic credit equals its declared hook amount; wallet authorization is gross.
//! See `skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md` and `docs/route-solver.md`.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::api::route_solver::RouteHop;
use crate::api::AppState;
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
    pub route_tax_code_ids: HashSet<i64>,
    pub route_tax_data_hashes: HashSet<String>,
    pub tax_free_code_ids: HashSet<i64>,
}

impl RouterHopTaxPolicy {
    pub fn from_catalog(cfg: &crate::config::CommunityTaxCatalogConfig) -> Self {
        Self {
            option2_code_ids: cfg.option2_code_ids.clone(),
            option2_data_hashes: cfg.option2_data_hashes.clone(),
            route_tax_code_ids: cfg.route_tax_code_ids.clone(),
            route_tax_data_hashes: cfg.route_tax_data_hashes.clone(),
            tax_free_code_ids: cfg.tax_free_code_ids.clone(),
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
    pub route_funding_version: Option<u16>,
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
            buy_bps: row.buy_bps.unwrap_or(0).clamp(0, 10_000) as u16,
            sell_bps: row.sell_bps.unwrap_or(0).clamp(0, 10_000) as u16,
            exemption_directory,
            route_funding_version: None,
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
    /// Exemption belongs to one token only; never inherited by another hop.
    pub exempt_tokens: HashSet<String>,
    pub unresolved: HashSet<String>,
    pub instance_identity: Vec<String>,
    pub router_funding_supported: bool,
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
    let bps = u128::from(buy_bps.min(10_000));
    let tax = (raw_out / TAX_BPS_DENOM) * bps + (raw_out % TAX_BPS_DENOM) * bps / TAX_BPS_DENOM;
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

/// The logical route visits its initial offer and each hop ask once. Shared offer/ask
/// boundaries between adjacent hops are one visit. Launch guards are stateful: v1
/// previews cannot promise parity when the same capable token is visited twice (#1390),
/// even with zero current rates or a manager/protocol exemption.
pub fn repeated_route_tax_asset<'a>(
    hops: &'a [RouteHop],
    snapshot: &TaxRankSnapshot,
) -> Option<&'a str> {
    let first = hops.first()?;
    let mut visited = HashSet::new();
    for asset in std::iter::once(first.offer_token.as_str())
        .chain(hops.iter().map(|hop| hop.ask_token.as_str()))
    {
        let normalized = asset.trim().to_ascii_lowercase();
        if snapshot
            .by_addr
            .get(&normalized)
            .is_some_and(|token| token.route_funding_version == Some(1))
            && !visited.insert(normalized)
        {
            return Some(asset);
        }
    }
    None
}

/// Reject unresolved implementations and incompatible funding versions at every boundary.
/// Supported v1 community intermediates remain eligible; ordinary implementations are attested.
pub fn path_sells_middle_tax_hop(
    hops: &[RouteHop],
    token_in: &str,
    snapshot: &TaxRankSnapshot,
) -> bool {
    if repeated_route_tax_asset(hops, snapshot).is_some() {
        return true;
    }
    if snapshot.path_route_local(hops) && !snapshot.router_funding_supported {
        return true;
    }
    let tin = token_in.trim().to_ascii_lowercase();
    for hop in hops {
        let offer = hop.offer_token.trim().to_ascii_lowercase();
        if snapshot.unresolved.contains(&offer)
            || snapshot
                .unresolved
                .contains(&hop.ask_token.to_ascii_lowercase())
        {
            return true;
        }
        if snapshot.by_addr.get(&offer).is_some_and(|t| {
            t.route_funding_version.is_none() && t.sell_bps > 0 && snapshot.router_funding_supported
        }) {
            return true;
        }
        if offer == tin {
            continue;
        }
        if snapshot.unresolved.contains(&offer)
            || snapshot
                .unresolved
                .contains(&hop.ask_token.to_ascii_lowercase())
        {
            return true;
        }
        let Some(row) = snapshot.by_addr.get(&offer) else {
            continue;
        };
        if row.route_funding_version == Some(1) && snapshot.router_funding_supported {
            continue;
        }
        if row.router_hops_tax(&snapshot.policy) || row.buy_bps > 0 || row.sell_bps > 0 {
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
    if snapshot.path_route_local(hops) {
        let eligible = !path_sells_middle_tax_hop(hops, token_in, snapshot);
        return TaxRankResult { raw_out, net_out: raw_out, eligible,
            buy_tax_bps: snapshot.token_out.as_ref().map(|t| snapshot.buy_bps(&t.contract_address)).unwrap_or(0),
            sell_tax_bps: snapshot.token_in.as_ref().map(|t| snapshot.sell_bps(&t.contract_address)).unwrap_or(0),
            tax_kind: "route_local_v1",
            tax_notes: "Route-local v1: authorized gross input; all hop taxes included exactly once in simulated net output.".into(),
            router_hops_tax: true };
    }
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
    let buy_tax_bps = if out_row.is_some_and(|t| snapshot.is_exempt(&t.contract_address)) {
        0
    } else {
        buy_catalog
    };
    let sell_tax_bps = if in_row.is_some_and(|t| snapshot.is_exempt(&t.contract_address)) {
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
            "Unsupported or unavailable trading tax capability; migrate/register compatible route-local funding.".into(),
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
    if !snapshot.has_catalog_tax()
        && snapshot.unresolved.is_empty()
        && snapshot.instance_identity.is_empty()
        && !snapshot.policy_has_option2()
    {
        return TAX_CACHE_IDENTITY_NONE.into();
    }
    let mut entries: Vec<_> = snapshot
        .by_addr
        .iter()
        .map(|(addr, t)| {
            format!(
                "{}:c{:?}:h{:?}:v{:?}:b{}:s{}:e{}:rh{}",
                addr,
                t.code_id,
                t.data_hash,
                t.route_funding_version,
                t.buy_bps,
                t.sell_bps,
                snapshot.is_exempt(addr),
                t.router_hops_tax(&snapshot.policy)
            )
        })
        .collect();
    entries.extend(snapshot.instance_identity.clone());
    entries.extend(
        snapshot
            .unresolved
            .iter()
            .map(|a| format!("{}:unresolved", a)),
    );
    entries.sort();
    // Identity contains protocol facts rather than time, so long-running progress remains
    // observable across discovery refreshes. Funded quote cache TTL is bounded separately.
    format!(
        "v1:r{}:{}",
        snapshot.router_funding_supported,
        entries.join("|")
    )
}

/// Largest economic sell funded by gross, preserving tax's economic rate basis.
pub fn sell_economic(gross: u128, bps: u16) -> u128 {
    let mut lo = 0u128;
    let mut hi = gross;
    while lo < hi {
        let mid = lo + (hi - lo) / 2 + (hi - lo) % 2;
        let tax = tax_amount(mid, bps);
        if mid.checked_add(tax).is_some_and(|total| total <= gross) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}
pub fn tax_amount(amount: u128, bps: u16) -> u128 {
    (amount / TAX_BPS_DENOM) * u128::from(bps.min(10_000))
        + (amount % TAX_BPS_DENOM) * u128::from(bps.min(10_000)) / TAX_BPS_DENOM
}
impl TaxRankSnapshot {
    pub fn is_exempt(&self, addr: &str) -> bool {
        let addr = addr.to_ascii_lowercase();
        self.exempt_tokens.contains(&addr)
            || (self.trader_exempt
                && self.exempt_tokens.is_empty()
                && !self
                    .by_addr
                    .get(&addr)
                    .is_some_and(|t| t.route_funding_version == Some(1)))
    }
    pub fn path_route_local(&self, hops: &[RouteHop]) -> bool {
        hops.iter().any(|h| {
            [&h.offer_token, &h.ask_token].iter().any(|a| {
                self.by_addr
                    .get(&a.to_ascii_lowercase())
                    .is_some_and(|t| t.route_funding_version == Some(1))
            })
        })
    }
    pub fn sell_bps(&self, addr: &str) -> u16 {
        if self.is_exempt(addr) {
            return 0;
        }
        self.by_addr
            .get(&addr.to_ascii_lowercase())
            .filter(|t| t.route_funding_version == Some(1))
            .map(|t| t.sell_bps)
            .unwrap_or(0)
    }
    pub fn buy_bps(&self, addr: &str) -> u16 {
        if self.is_exempt(addr) {
            return 0;
        }
        self.by_addr
            .get(&addr.to_ascii_lowercase())
            .filter(|t| t.route_funding_version == Some(1))
            .map(|t| t.buy_bps)
            .unwrap_or(0)
    }
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
    try_load_tax_rank_snapshot(state, token_in, token_out, extra_addrs, trader)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "trading tax discovery failed; route unavailable");
            TaxRankSnapshot {
                policy: RouterHopTaxPolicy::from_catalog(&state.community_tax),
                unresolved: [token_in, token_out]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                ..Default::default()
            }
        })
}

pub(crate) async fn try_load_tax_rank_snapshot(
    state: &AppState,
    token_in: &str,
    token_out: &str,
    extra_addrs: &[String],
    trader: Option<&str>,
) -> Result<TaxRankSnapshot, sqlx::Error> {
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

    use super::trading_tax_discovery::{self, TradingTax};
    let mut snapshot = TaxRankSnapshot {
        instance_identity: vec![format!(
            "lcd{}:router{:?}",
            state.lcd.chain_cache_namespace(),
            state.router_address
        )],
        policy,
        ..Default::default()
    };
    if addrs.len() > trading_tax_discovery::MAX_DISCOVERY_ASSETS {
        snapshot.unresolved.extend(addrs);
        return Ok(snapshot);
    }
    // Catalog is supplemental evidence only. Its failure cannot erase actual trading metadata.
    let rows = community_tokens::get_by_addresses(&state.pool, &addrs)
        .await
        .unwrap_or_default();
    let mut tasks = tokio::task::JoinSet::new();
    for addr in addrs {
        let state = state.clone();
        let policy = snapshot.policy.clone();
        tasks.spawn(async move {
            let discovered = trading_tax_discovery::discover(&state, &addr, &policy).await;
            (addr, discovered)
        });
    }
    while let Some(result) = tasks.join_next().await {
        let (addr, discovered) = result.map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        match discovered {
            TradingTax::Ordinary { code_id } => snapshot
                .instance_identity
                .push(format!("{}:ordinary:{:?}", addr, code_id)),
            TradingTax::Community(token) => {
                // Any wallet-funded legacy charge is containment only, never advertised v1.
                if token.route_funding_version.is_none() {
                    snapshot.unresolved.insert(addr.clone());
                }
                snapshot.by_addr.insert(addr, token);
            }
            TradingTax::Unsupported | TradingTax::Unavailable => {
                snapshot.unresolved.insert(addr);
            }
        }
    }
    for row in rows {
        let addr = row.contract_address.trim().to_ascii_lowercase();
        if !snapshot.by_addr.contains_key(&addr) && snapshot.unresolved.contains(&addr) {
            snapshot
                .by_addr
                .insert(addr, CataloguedTaxToken::from_row(&row));
        }
    }
    snapshot.token_in = snapshot
        .by_addr
        .get(&token_in.trim().to_ascii_lowercase())
        .cloned();
    snapshot.token_out = snapshot
        .by_addr
        .get(&token_out.trim().to_ascii_lowercase())
        .cloned();
    snapshot.router_funding_supported = if snapshot
        .by_addr
        .values()
        .any(|t| t.route_funding_version == Some(1))
    {
        trading_tax_discovery::router_funding_supported(state).await
    } else {
        false
    };
    if let Some(trader) = trader.map(str::trim).filter(|s| !s.is_empty()) {
        let mut queries = tokio::task::JoinSet::new();
        for token in snapshot.by_addr.values().filter(|t| t.exemption_directory) {
            let lcd = state.lcd.clone();
            let token = token.clone();
            let trader = trader.to_owned();
            queries.spawn(async move {
                let exempt = resolve_token_exempt(&lcd, &token, &trader).await;
                (token.contract_address, exempt)
            });
        }
        while let Some(Ok((addr, exempt))) = queries.join_next().await {
            if exempt {
                snapshot.exempt_tokens.insert(addr);
            }
        }
    }
    Ok(snapshot)
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
async fn resolve_token_exempt(lcd: &LcdClient, token: &CataloguedTaxToken, trader: &str) -> bool {
    let query = serde_json::json!({"is_protocol_exempt":{"address":trader}});
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let _permit = super::trading_tax_discovery::discovery_semaphore()
            .acquire()
            .await;
        lcd.query_contract::<IsExemptLcd>(&token.contract_address, &query)
            .await
    })
    .await
    .ok()
    .and_then(Result::ok)
    .is_some_and(|v| v.manager || (token.route_funding_version != Some(1) && v.protocol))
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
            route_funding_version: None,
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
            ..Default::default()
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
    fn legacy_taxed_middle_hop_is_contained_until_registered_upgrade() {
        let t = tax_tok(TAX, COLUMBUS5_COMMUNITY_TAX_CODE_ID, 100, 200);
        let mut s = snap(None, None, RouterHopTaxPolicy::default(), false);
        s.by_addr.insert(TAX.to_string(), t);
        let via_tax = vec![hop(UST1, TAX), hop(TAX, USTR)];
        let r = score_path(5_000_000, &via_tax, UST1, &s);
        assert!(
            !r.eligible,
            "unknown legacy funding cannot attest route-local execution"
        );
        assert_eq!(r.net_out, 5_000_000);
        assert_eq!(r.tax_kind, "skipped_middle_sell");
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
    #[test]
    fn funded_sell_uses_maximal_economic_integer_and_no_overflow() {
        assert_eq!(sell_economic(1_010_000, 100), 1_000_000);
        for bps in [0, 1, 100, 450, 10_000] {
            for gross in [1, 99, 100, 10_000, 1_010_000, u128::MAX] {
                let economic = sell_economic(gross, bps);
                assert!(economic.checked_add(tax_amount(economic, bps)).unwrap() <= gross);
                if let Some(next) = economic.checked_add(1) {
                    assert!(next
                        .checked_add(tax_amount(next, bps))
                        .is_none_or(|total| total > gross));
                }
            }
        }
    }

    #[test]
    fn upgraded_intermediate_eligible_and_net_simulation_not_haircut_twice() {
        let mut s = TaxRankSnapshot {
            router_funding_supported: true,
            ..Default::default()
        };
        let mut token = tax_tok(TAX, 11667, 450, 100);
        token.route_funding_version = Some(1);
        s.by_addr.insert(TAX.into(), token.clone());
        let hops = [hop(UST1, TAX), hop(TAX, USTR)];
        let result = score_path(955_000, &hops, UST1, &s);
        assert!(result.eligible);
        assert_eq!(result.net_out, 955_000);
        assert_eq!(result.tax_kind, "route_local_v1");
        s.token_out = Some(token);
        let result = score_path(955_000, &[hop(UST1, TAX)], UST1, &s);
        assert_eq!(result.buy_tax_bps, 450);
        assert_eq!(result.net_out, result.raw_out);
        s.router_funding_supported = false;
        assert!(!score_path(955_000, &hops, UST1, &s).eligible);
    }

    #[test]
    fn token_exemption_and_intermediate_migration_config_are_cache_identity() {
        let mut s = TaxRankSnapshot {
            router_funding_supported: true,
            ..Default::default()
        };
        let mut a = tax_tok(TAX, 11667, 450, 100);
        a.route_funding_version = Some(1);
        let mut b = tax_tok(USTR, 11667, 300, 200);
        b.route_funding_version = Some(1);
        s.by_addr.insert(TAX.into(), a);
        s.by_addr.insert(USTR.into(), b);
        let initial = s.cache_identity();
        s.exempt_tokens.insert(TAX.into());
        assert_eq!(s.sell_bps(TAX), 0);
        assert_eq!(s.sell_bps(USTR), 200);
        assert_ne!(s.cache_identity(), initial);
        let exempt = s.cache_identity();
        s.by_addr.get_mut(TAX).unwrap().code_id = Some(11668);
        assert_ne!(s.cache_identity(), exempt);
        let migrated = s.cache_identity();
        s.by_addr.get_mut(TAX).unwrap().buy_bps = 500;
        assert_ne!(s.cache_identity(), migrated);
        s.unresolved.insert(TAX.into());
        assert!(path_sells_middle_tax_hop(
            &[hop(UST1, TAX), hop(TAX, USTR)],
            UST1,
            &s
        ));
    }
    #[test]
    fn repeated_capable_assets_rejected_independent_of_rates_and_exemptions() {
        for (buy, sell) in [(0, 0), (450, 100)] {
            let mut token = tax_tok(TAX, 11667, buy, sell);
            token.route_funding_version = Some(1);
            let mut s = snap(Some(token), None, Default::default(), true);
            s.router_funding_supported = true;
            s.exempt_tokens.insert(TAX.into());
            for hops in [
                vec![hop(TAX, UST1), hop(UST1, TAX)],
                vec![hop(UST1, TAX), hop(TAX, USTR), hop(USTR, TAX)],
            ] {
                assert_eq!(repeated_route_tax_asset(&hops, &s), Some(TAX));
                assert!(!score_path(1_000_000, &hops, &hops[0].offer_token, &s).eligible);
            }
            // Adjacent ask/offer references are a single logical visit.
            let simple = [hop(UST1, TAX), hop(TAX, USTR)];
            assert_eq!(repeated_route_tax_asset(&simple, &s), None);
            assert!(score_path(1_000_000, &simple, UST1, &s).eligible);
            // Ordinary repeated assets remain supported when the capable asset is unique.
            let ordinary_cycle = [hop(UST1, TAX), hop(TAX, USTR), hop(USTR, UST1)];
            assert!(score_path(1_000_000, &ordinary_cycle, UST1, &s).eligible);
            // Discovery failure must reject before a cycle can be treated as ordinary.
            s.unresolved.insert(TAX.into());
            assert!(!score_path(1_000_000, &simple, UST1, &s).eligible);
        }
    }

    #[tokio::test]
    async fn route_local_trader_protocol_flag_cannot_waive_tax() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path_regex(r".*/smart/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data":{"manager":false,"protocol":true}
            })))
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
        let mut token = tax_tok(TAX, 11667, 450, 100);
        token.route_funding_version = Some(1);
        assert!(!resolve_token_exempt(&lcd, &token, "terra1synthetictrader").await);
        let mut snapshot = TaxRankSnapshot {
            trader_exempt: true,
            router_funding_supported: true,
            ..Default::default()
        };
        snapshot.by_addr.insert(TAX.into(), token);
        assert_eq!(
            snapshot.sell_bps(TAX),
            100,
            "legacy global exemption cannot waive v1 trader tax"
        );
        assert_eq!(snapshot.buy_bps(TAX), 450);
        snapshot.exempt_tokens.insert(TAX.into());
        assert_eq!(
            snapshot.sell_bps(TAX),
            0,
            "explicit per-token manager exemption remains supported"
        );
    }
}
