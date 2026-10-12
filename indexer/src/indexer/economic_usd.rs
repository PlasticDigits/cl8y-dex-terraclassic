//! Factory-listed economic CW20 USD marks for protocol fee ingest (GitLab #683).
//!
//! After hub bootstrap (cUSTC / LUNC / UST1 / USTR), walk factory `pair_reserves`
//! for pairs whose **one** leg is already hub/catalog-priced and the **other** is a
//! factory-listed economic CW20. Winner = max humanized USD-TVL, then lex pair address.
//!
//! Identity is **contract** (A1). Never `$1` CL8Y, CoinGecko, CEX FDUSD, or Venus.
//! Gems (#562 addr + ticker) and Terra vFDUSD stay unpriced. Not exposed on GET /hub-prices.

use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};

use super::defillama::COLUMBUS5_GEM_ADDRESSES;
use super::hub_usd::{
    is_hub_custc, is_hub_ust1, is_hub_ustr, is_native_denom, is_stale, pair_tvl, reserves_usable,
    same_asset, usd_from_reserves, AssetRef, HubMark, HubUsdConfig, HubUsdSnapshot, ReservePair,
};
use crate::config::DEFAULT_VFDUSD_ADDRESS;

/// Soft-launch / LocalTerra gem tickers (#562). Addr set wins for columbus-5; ticker
/// catches LocalTerra faucet gems whose contracts are not in `COLUMBUS5_GEM_ADDRESSES`.
pub const GEM_TICKERS: &[&str] = &[
    "EMBER", "CORAL", "JADE", "ONYX", "RUBY", "TOPAZ", "QUARTZ", "PEARL", "OPAL", "COBALT",
    "SLATE", "AMBER", "IRON",
];

#[derive(Debug, Clone, PartialEq)]
pub struct EconomicMark {
    pub contract_address: String,
    pub asset_id: i32,
    pub price_usd: BigDecimal,
    pub source_pair_id: i32,
    pub source_pair_address: String,
    pub tvl_usd: BigDecimal,
    pub as_of: DateTime<Utc>,
    pub anchor_ticker: String,
}

fn addr_eq(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

fn is_hub_clunc(asset: &AssetRef, cfg: &HubUsdConfig) -> bool {
    if is_native_denom(asset, "uluna") {
        return true;
    }
    asset.is_cw20
        && asset.denom.is_none()
        && asset
            .contract_address
            .as_deref()
            .is_some_and(|a| addr_eq(a, &cfg.clunc_address))
}

/// Gem by columbus-5 address (not symbol). Spoof `symbol=UST1` on a gem addr stays a gem.
pub fn is_gem_address(contract: &str) -> bool {
    let lower = contract.trim().to_ascii_lowercase();
    COLUMBUS5_GEM_ADDRESSES
        .iter()
        .any(|a| a.eq_ignore_ascii_case(&lower))
}

pub fn is_gem_ticker(symbol: &str) -> bool {
    let n = normalize_symbol(symbol);
    GEM_TICKERS.iter().any(|t| *t == n)
}

/// Terra CW20 vFDUSD / FDUSD display — never CEX FDUSD or Venus (X4 / #580).
pub fn is_vfdusd_identity(symbol: &str, contract: Option<&str>) -> bool {
    match normalize_symbol(symbol).as_str() {
        "VFDUSD" | "FDUSD" => return true,
        _ => {}
    }
    contract
        .map(|c| addr_eq(c, DEFAULT_VFDUSD_ADDRESS))
        .unwrap_or(false)
}

pub fn is_hub_wrap(asset: &AssetRef, cfg: &HubUsdConfig) -> bool {
    is_hub_custc(asset, cfg)
        || is_hub_clunc(asset, cfg)
        || is_hub_ust1(asset, cfg)
        || is_hub_ustr(asset, cfg)
}

/// Official CL8Y CW20 pin (`HUB_CL8Y_ADDRESS`). Identity is contract, not ticker.
pub fn is_official_cl8y(asset: &AssetRef, cfg: &HubUsdConfig) -> bool {
    asset.is_cw20
        && asset
            .contract_address
            .as_deref()
            .is_some_and(|a| addr_eq(a, &cfg.cl8y_address))
}

/// Factory-listed economic CW20 eligible for a reserve mark.
///
/// Requires CW20 + contract. Excludes gems (addr + ticker), vFDUSD, and hub wraps.
/// Official CL8Y is the pin; other listed non-gems use the same path (not `symbol == CL8Y`).
pub fn is_economic_fee_token(asset: &AssetRef, cfg: &HubUsdConfig) -> bool {
    if !asset.is_cw20 {
        return false;
    }
    let Some(addr) = asset.contract_address.as_deref() else {
        return false;
    };
    let addr = addr.trim();
    if addr.is_empty() || !addr.starts_with("terra1") {
        return false;
    }
    if is_hub_wrap(asset, cfg) {
        return false;
    }
    if is_gem_address(addr) {
        return false;
    }
    if is_vfdusd_identity(&asset.symbol, Some(addr)) {
        return false;
    }
    if is_official_cl8y(asset, cfg) {
        return true;
    }
    if is_gem_ticker(&asset.symbol) {
        return false;
    }
    true
}

fn priced_hub_usd(
    asset: &AssetRef,
    cfg: &HubUsdConfig,
    snap: &HubUsdSnapshot,
) -> Option<BigDecimal> {
    priced_hub_mark(asset, cfg, snap).map(|mark| mark.price_usd.clone())
}

fn priced_hub_mark<'a>(
    asset: &AssetRef,
    cfg: &HubUsdConfig,
    snap: &'a HubUsdSnapshot,
) -> Option<&'a HubMark> {
    if is_hub_custc(asset, cfg) {
        return snap.custc.as_ref();
    }
    if is_hub_clunc(asset, cfg) {
        return snap.lunc.as_ref();
    }
    if is_hub_ust1(asset, cfg) {
        return snap.ust1.as_ref();
    }
    if is_hub_ustr(asset, cfg) {
        return snap.ustr.as_ref();
    }
    None
}

fn usable_pair(now: DateTime<Utc>, cfg: &HubUsdConfig, pair: &ReservePair) -> bool {
    if same_asset(&pair.asset_0, &pair.asset_1) {
        return false;
    }
    if !reserves_usable(pair) {
        return false;
    }
    if is_stale(now, pair.snapshot_at, cfg.max_staleness) {
        return false;
    }
    true
}

#[derive(Clone)]
struct Candidate {
    contract_address: String,
    pair_id: i32,
    pair_address: String,
    price_usd: BigDecimal,
    tvl_usd: BigDecimal,
    as_of: DateTime<Utc>,
    anchor_ticker: String,
    asset_id: i32,
}

fn pick_best(mut cands: Vec<Candidate>) -> Option<Candidate> {
    cands.sort_by(|a, b| match b.tvl_usd.cmp(&a.tvl_usd) {
        std::cmp::Ordering::Equal => a.pair_address.cmp(&b.pair_address),
        o => o,
    });
    cands.into_iter().next()
}

/// Resolve one mark per economic CW20 from factory pairs vs already-priced hub legs.
///
/// Single hop only — the other leg must already be catalog/hub-priced. No walk of
/// unpriced economic ↔ unpriced economic (circular quote).
pub fn resolve_economic_marks(
    now: DateTime<Utc>,
    cfg: &HubUsdConfig,
    pairs: &[ReservePair],
    snap: &HubUsdSnapshot,
) -> Vec<EconomicMark> {
    let mut by_contract: HashMap<String, Vec<Candidate>> = HashMap::new();

    for pair in pairs {
        if !usable_pair(now, cfg, pair) {
            continue;
        }
        let orientations = [
            (
                &pair.asset_0,
                &pair.asset_1,
                &pair.reserve_0,
                &pair.reserve_1,
                true,
            ),
            (
                &pair.asset_1,
                &pair.asset_0,
                &pair.reserve_1,
                &pair.reserve_0,
                false,
            ),
        ];
        for (token, other, r_token, r_other, token_is_0) in orientations {
            if !is_economic_fee_token(token, cfg) {
                continue;
            }
            let Some(other_usd) = priced_hub_usd(other, cfg, snap) else {
                continue;
            };
            let Some(token_usd) =
                usd_from_reserves(token.decimals, other.decimals, r_token, r_other, &other_usd)
            else {
                continue;
            };
            let usd_0 = if token_is_0 { &token_usd } else { &other_usd };
            let usd_1 = if token_is_0 { &other_usd } else { &token_usd };
            let Some(tvl) = pair_tvl(pair, usd_0, usd_1) else {
                continue;
            };
            if tvl < cfg.economic_tvl_floor {
                continue;
            }
            let Some(contract) = token.contract_address.as_deref() else {
                continue;
            };
            let key = contract.trim().to_ascii_lowercase();
            by_contract.entry(key.clone()).or_default().push(Candidate {
                contract_address: key,
                pair_id: pair.pair_id,
                pair_address: pair.pair_address.clone(),
                price_usd: token_usd,
                tvl_usd: tvl,
                anchor_ticker: if is_hub_custc(other, cfg) {
                    "custc"
                } else if is_hub_clunc(other, cfg) {
                    "lunc"
                } else if is_hub_ust1(other, cfg) {
                    "ust1"
                } else {
                    "ustr"
                }
                .into(),
                as_of: [
                    (&snap.custc, is_hub_custc(other, cfg)),
                    (&snap.lunc, is_hub_clunc(other, cfg)),
                    (&snap.ust1, is_hub_ust1(other, cfg)),
                    (&snap.ustr, is_hub_ustr(other, cfg)),
                ]
                .into_iter()
                .find(|(_, matches)| *matches)
                .and_then(|(mark, _)| mark.as_ref().and_then(|m| m.as_of))
                .map(|at| at.min(pair.snapshot_at))
                .unwrap_or(pair.snapshot_at),
                asset_id: token.id,
            });
        }
    }

    let mut out = Vec::new();
    for (_k, cands) in by_contract {
        if let Some(best) = pick_best(cands) {
            out.push(EconomicMark {
                contract_address: best.contract_address,
                asset_id: best.asset_id,
                price_usd: best.price_usd,
                source_pair_id: best.pair_id,
                source_pair_address: best.pair_address,
                tvl_usd: best.tvl_usd,
                as_of: best.as_of,
                anchor_ticker: best.anchor_ticker,
            });
        }
    }
    out.sort_by(|a, b| a.contract_address.cmp(&b.contract_address));
    out
}

/// Diagnose the reporting-floor coupling without admitting a new price class.
/// A complete candidate census is required by callers: a deeper eligible pool
/// must not be misclassified because a bounded partial census saw only dust.
/// Returned values are evidence only and must never enter executable references.
pub fn economic_floor_exclusions(
    now: DateTime<Utc>,
    cfg: &HubUsdConfig,
    pairs: &[ReservePair],
    snap: &HubUsdSnapshot,
) -> Vec<(i32, DateTime<Utc>)> {
    let mut diagnostic = cfg.clone();
    diagnostic.economic_tvl_floor = BigDecimal::from(0);
    diagnostic.max_staleness = std::time::Duration::from_secs(20);
    let mut qualified = snap.clone();
    for mark in [
        &mut qualified.custc,
        &mut qualified.lunc,
        &mut qualified.ust1,
        &mut qualified.ustr,
    ] {
        if mark.as_ref().is_some_and(|mark| {
            mark.price_usd <= BigDecimal::from(0)
                || mark.as_of.is_none_or(|at| {
                    at > now || now.signed_duration_since(at).num_milliseconds() > 20_000
                })
        }) {
            *mark = None;
        }
    }
    resolve_economic_marks(now, &diagnostic, pairs, &qualified)
        .into_iter()
        .filter(|mark| {
            mark.tvl_usd < cfg.economic_tvl_floor
                && mark.as_of <= now
                && now.signed_duration_since(mark.as_of).num_milliseconds() <= 20_000
                // A missing/stale alternative may later publish an eligible
                // reference. Its current absence remains recoverable data loss.
                && pairs.iter().all(|pair| {
                    !((pair.asset_0.id == mark.asset_id && is_hub_wrap(&pair.asset_1, cfg))
                        || (pair.asset_1.id == mark.asset_id && is_hub_wrap(&pair.asset_0, cfg)))
                        || (usable_pair(now, &diagnostic, pair)
                            && priced_hub_usd(
                                if pair.asset_0.id == mark.asset_id { &pair.asset_1 } else { &pair.asset_0 },
                                cfg,
                                &qualified,
                            ).is_some())
                })
        })
        .map(|mark| {
            let oldest = pairs.iter().filter_map(|pair| {
                let other = if pair.asset_0.id == mark.asset_id { &pair.asset_1 }
                    else if pair.asset_1.id == mark.asset_id { &pair.asset_0 }
                    else { return None; };
                priced_hub_mark(other, cfg, &qualified)
                    .and_then(|anchor| anchor.as_of)
                    .map(|at| at.min(pair.snapshot_at))
            }).fold(mark.as_of, |oldest, at| oldest.min(at));
            (mark.asset_id, oldest)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::hub_usd::{resolve_hub_usd, HubMark, HubTicker};
    use std::str::FromStr;
    use std::time::Duration;

    #[test]
    fn issue1479_malformed_native_cw20_aliases_cannot_anchor_producer_or_diagnosis() {
        let now = Utc::now();
        let mut cfg = cfg();
        cfg.economic_tvl_floor = bd("10");
        let token = cw20(20, "SMALL", 6, "terra1small");
        let mut snap = snap_with_hub(now, &[]);
        snap.lunc.as_mut().unwrap().as_of = Some(now);
        snap.custc.as_mut().unwrap().as_of = Some(now);
        for (denom, wrapped) in [
            ("uluna", cfg.clunc_address.as_str()),
            ("uusd", cfg.custc_address.as_str()),
        ] {
            let canonical = native(7, "spoofed-symbol", denom, 6);
            let positive = pair(
                20,
                "terra1nativefixture",
                token.clone(),
                canonical.clone(),
                "100000000000",
                "1000000000000",
                now,
            );
            assert_eq!(
                resolve_economic_marks(now, &cfg, &[positive.clone()], &snap).len(),
                1
            );
            for mut malformed in [canonical.clone(), cw20(7, "spoofed-symbol", 6, wrapped)] {
                if malformed.is_cw20 {
                    malformed.denom = Some(denom.into());
                } else if malformed.contract_address.is_none() {
                    malformed.contract_address = Some("terra1spoof".into());
                }
                let negative = pair(
                    20,
                    "terra1nativefixture",
                    token.clone(),
                    malformed,
                    "100000000000000",
                    "1000000000",
                    now,
                );
                assert!(resolve_economic_marks(now, &cfg, &[negative.clone()], &snap).is_empty());
                assert!(economic_floor_exclusions(now, &cfg, &[negative], &snap).is_empty());
            }
            let mut contradictory = canonical;
            contradictory.is_cw20 = true;
            let negative = pair(
                20,
                "terra1nativefixture",
                token.clone(),
                contradictory,
                "100000000000000",
                "1000000000",
                now,
            );
            assert!(resolve_economic_marks(now, &cfg, &[negative.clone()], &snap).is_empty());
            assert!(economic_floor_exclusions(now, &cfg, &[negative], &snap).is_empty());
        }
    }

    #[test]
    fn issue1479_separate_ten_dollar_total_tvl_policy_has_exact_boundaries() {
        let now = Utc::now();
        let mut cfg = cfg();
        cfg.economic_tvl_floor = bd(crate::config::DEFAULT_ECONOMIC_USD_TVL_FLOOR);
        assert_eq!(cfg.tvl_floor, bd(crate::config::DEFAULT_HUB_USD_TVL_FLOOR));
        let token = cw20(20, "SMALL", 6, "terra1small");
        let anchor = cw20(7, "cLUNC", 6, "terra1clunc");
        let mut snap = snap_with_hub(now, &[]);
        snap.lunc.as_mut().unwrap().as_of = Some(now);
        for anchor_raw in [
            62_499_999_999u128,
            62_500_000_000,
            62_500_000_001,
            156_250_000_000,
        ] {
            let pair = pair(
                20,
                "terra1smallpair",
                token.clone(),
                anchor.clone(),
                "100000000000",
                &anchor_raw.to_string(),
                now,
            );
            let marks = resolve_economic_marks(now, &cfg, &[pair.clone()], &snap);
            let excluded = economic_floor_exclusions(now, &cfg, &[pair], &snap);
            if anchor_raw < 62_500_000_000 {
                assert!(marks.is_empty());
                assert_eq!(excluded, vec![(20, now)]);
            } else {
                assert_eq!(marks.len(), 1);
                let expected_total =
                    BigDecimal::from(anchor_raw) * bd("0.00008") * BigDecimal::from(2)
                        / BigDecimal::from(1_000_000);
                assert_eq!(marks[0].tvl_usd, expected_total);
                assert_eq!(marks[0].as_of, now);
                assert!(excluded.is_empty());
            }
        }
        // Economic eligibility does not promote a shallow UST1 pool to trusted hub.
        let shallow_hub = pair(
            30,
            "terra1shallowhub",
            cw20(1, "UST1", 6, "terra1ust1"),
            cw20(2, "cUSTC", 6, "terra1custc"),
            "10000000",
            "1000000000",
            now,
        );
        assert!(
            resolve_hub_usd(now, &cfg, Some(&bd("0.005")), &[shallow_hub], Some(2))
                .ust1
                .is_none()
        );
        assert_eq!(cfg.economic_tvl_floor, bd("10"));
        assert_eq!(cfg.tvl_floor, bd("100"));
    }

    #[test]
    fn issue1479_total_tvl_floor_diagnosis_is_not_independent_depth_or_price_admission() {
        let now = Utc::now();
        let cfg = cfg();
        let token = cw20(20, "SMALL", 6, "terra1small");
        let anchor = cw20(7, "cLUNC", 6, "terra1clunc");
        let mut snap = snap_with_hub(now, &[]);
        snap.lunc.as_mut().unwrap().as_of = Some(now);
        // Independently valued anchor leg = 156250 LUNC * $0.00008 = $12.50.
        // Self-imputed token leg = $12.50; total-TVL = $25, not $12.50 depth.
        let small = pair(
            20,
            "terra1smallpair",
            token.clone(),
            anchor.clone(),
            "100000000000",
            "156250000000",
            now,
        );
        assert!(resolve_economic_marks(now, &cfg, &[small.clone()], &snap).is_empty());
        assert_eq!(
            economic_floor_exclusions(now, &cfg, &[small.clone()], &snap),
            vec![(20, now)]
        );
        // $100 total TVL is admitted despite only $50 independently priced depth:
        // preserve the existing reporting policy; do not silently change units.
        let at_floor = pair(
            21,
            "terra1largerpair",
            token,
            anchor,
            "400000000000",
            "625000000000",
            now,
        );
        let marks = resolve_economic_marks(now, &cfg, &[at_floor.clone()], &snap);
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].tvl_usd, bd("100"));
        assert!(economic_floor_exclusions(now, &cfg, &[small, at_floor], &snap).is_empty());
        assert_eq!(cfg.tvl_floor, bd("100"));
    }

    #[test]
    fn issue1479_policy_diagnosis_requires_original_fresh_complete_candidates_and_anchor() {
        let now = Utc::now();
        let cfg = cfg();
        let small = pair(
            20,
            "terra1smallpair",
            cw20(20, "SMALL", 18, "terra1small"),
            cw20(7, "cLUNC", 6, "terra1clunc"),
            "100000000000000000000000",
            "156250000000",
            now,
        );
        let mut snap = snap_with_hub(now, &[]);
        assert!(economic_floor_exclusions(now, &cfg, &[small.clone()], &snap).is_empty());
        for anchor_at in [
            now - chrono::Duration::seconds(21),
            now + chrono::Duration::seconds(1),
        ] {
            snap.lunc.as_mut().unwrap().as_of = Some(anchor_at);
            assert!(economic_floor_exclusions(now, &cfg, &[small.clone()], &snap).is_empty());
        }
        snap.lunc.as_mut().unwrap().as_of = Some(now);
        let mut unqualified_alternative = small.clone();
        unqualified_alternative.asset_1 = cw20(2, "cUSTC", 6, "terra1custc");
        snap.custc = None;
        assert!(economic_floor_exclusions(
            now,
            &cfg,
            &[small.clone(), unqualified_alternative],
            &snap
        )
        .is_empty());
        let mut unavailable = small.clone();
        unavailable.snapshot_at = now - chrono::Duration::seconds(21);
        assert!(
            economic_floor_exclusions(now, &cfg, &[small.clone(), unavailable.clone()], &snap)
                .is_empty()
        );
        unavailable.snapshot_at = now;
        unavailable.reserve_0 = bd("0");
        assert!(
            economic_floor_exclusions(now, &cfg, &[small.clone(), unavailable], &snap).is_empty()
        );
        let mut circular = small.clone();
        circular.asset_1 = cw20(30, "cLUNC", 6, "terra1spoof");
        assert!(economic_floor_exclusions(now, &cfg, &[circular], &snap).is_empty());
        assert_eq!(
            economic_floor_exclusions(now, &cfg, &[small], &snap),
            vec![(20, now)]
        );
    }

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    fn cfg() -> HubUsdConfig {
        HubUsdConfig {
            custc_address: "terra1custc".into(),
            clunc_address: "terra1clunc".into(),
            ust1_address: "terra1ust1".into(),
            ustr_address: "terra1ustr".into(),
            cl8y_address: crate::config::DEFAULT_HUB_CL8Y_ADDRESS.to_string(),
            usdt_address: crate::config::DEFAULT_USDT_CW20_ADDRESS.to_string(),
            tvl_floor: bd("100"),
            economic_tvl_floor: bd("100"),
            max_staleness: Duration::from_secs(60),
        }
    }

    fn cw20(id: i32, symbol: &str, decimals: i16, contract: &str) -> AssetRef {
        AssetRef {
            id,
            symbol: symbol.into(),
            denom: None,
            contract_address: Some(contract.into()),
            is_cw20: true,
            decimals,
        }
    }

    fn native(id: i32, symbol: &str, denom: &str, decimals: i16) -> AssetRef {
        AssetRef {
            id,
            symbol: symbol.into(),
            denom: Some(denom.into()),
            contract_address: None,
            is_cw20: false,
            decimals,
        }
    }

    fn pair(
        id: i32,
        addr: &str,
        a0: AssetRef,
        a1: AssetRef,
        r0: &str,
        r1: &str,
        now: DateTime<Utc>,
    ) -> ReservePair {
        ReservePair {
            pair_id: id,
            pair_address: addr.into(),
            asset_0: a0,
            asset_1: a1,
            reserve_0: bd(r0),
            reserve_1: bd(r1),
            snapshot_at: now,
        }
    }

    fn usd_f(v: &BigDecimal) -> f64 {
        use bigdecimal::ToPrimitive;
        v.to_f64().unwrap()
    }

    fn snap_with_hub(now: DateTime<Utc>, extra: &[ReservePair]) -> HubUsdSnapshot {
        let c = cfg();
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ust1_pool = pair(
            11,
            "terra1deep",
            ust1,
            custc,
            "250000000",
            "50000000000",
            now,
        );
        let mut pairs = vec![ust1_pool];
        pairs.extend(extra.iter().cloned());
        let mut snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &pairs, Some(2));
        snap.lunc = Some(HubMark {
            ticker: HubTicker::Lunc,
            asset_id: Some(7),
            price_usd: bd("0.00008"),
            source_pair_id: None,
            source_pair_address: None,
            tvl_usd: None,
            priced_leg_depth_usd: None,
            as_of: None,
        });
        snap
    }

    #[test]
    fn cl8y_symbol_alone_is_not_economic_without_contract() {
        let c = cfg();
        let spoof = native(9, "CL8Y", "ucl8y", 6);
        assert!(!is_economic_fee_token(&spoof, &c));
        let cb = native(10, "CL8Y-cb", "ucl8y", 6);
        assert!(!is_economic_fee_token(&cb, &c));
    }

    #[test]
    fn pinned_cl8y_cw20_is_economic() {
        let c = cfg();
        let cl8y = cw20(4, "CL8Y-cb", 6, crate::config::DEFAULT_HUB_CL8Y_ADDRESS);
        assert!(is_economic_fee_token(&cl8y, &c));
        let other = cw20(
            5,
            "NEON",
            6,
            "terra1listedeconomicxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(is_economic_fee_token(&other, &c));
    }

    #[test]
    fn gem_addr_and_ticker_excluded() {
        let c = cfg();
        let ember = cw20(8, "UST1", 6, COLUMBUS5_GEM_ADDRESSES[0]);
        assert!(!is_economic_fee_token(&ember, &c));
        let local_gem = cw20(
            9,
            "EMBER",
            6,
            "terra1localemberxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(!is_economic_fee_token(&local_gem, &c));
    }

    #[test]
    fn vfdusd_excluded_by_symbol_and_pin() {
        let c = cfg();
        let v = cw20(3, "vFDUSD", 6, DEFAULT_VFDUSD_ADDRESS);
        assert!(!is_economic_fee_token(&v, &c));
        let fd = cw20(
            3,
            "FDUSD",
            6,
            "terra1fdusdxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(!is_economic_fee_token(&fd, &c));
        assert!(is_vfdusd_identity("VFDUSD", None));
    }

    #[test]
    fn hub_wraps_are_not_economic_marks() {
        let c = cfg();
        assert!(!is_economic_fee_token(
            &cw20(1, "UST1", 6, "terra1ust1"),
            &c
        ));
        assert!(!is_economic_fee_token(
            &cw20(2, "cUSTC", 6, "terra1custc"),
            &c
        ));
    }

    #[test]
    fn ranking_max_tvl_then_lex_skips_dust() {
        let now = Utc::now();
        let c = cfg();
        let cl8y = crate::config::DEFAULT_HUB_CL8Y_ADDRESS;
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let token = cw20(4, "CL8Y", 6, cl8y);
        // Dust: 1 human each vs UST1=$1 → TVL $2 < $100
        let dust = pair(
            40,
            "terra1dustcl8y",
            ust1.clone(),
            token.clone(),
            "1000000",
            "1000000",
            now,
        );
        // Deep vs UST1: 10_000 human CL8Y + 100 human UST1
        // usd(CL8Y)=1*100/10000=0.01; TVL=10000*0.01+100*1=$200
        let vs_ust1 = pair(
            41,
            "terra1cl8yust1",
            ust1,
            token.clone(),
            "100000000",
            "10000000000",
            now,
        );
        // Thinner vs cUSTC: 5_000 human CL8Y + 10_000 human cUSTC
        // usd(CL8Y)=0.005*10000/5000=0.01; TVL=5000*0.01+10000*0.005=$100
        let vs_custc = pair(
            42,
            "terra1cl8ycustc",
            token,
            cw20(2, "cUSTC", 6, "terra1custc"),
            "5000000000",
            "10000000000",
            now,
        );
        let extra = vec![dust.clone(), vs_ust1.clone(), vs_custc.clone()];
        let snap = snap_with_hub(now, &extra);
        let marks = resolve_economic_marks(now, &c, &extra, &snap);
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].source_pair_address, "terra1cl8yust1");
        assert!((usd_f(&marks[0].price_usd) - 0.01).abs() < 1e-9);
    }

    #[test]
    fn circular_unpriced_other_leg_skipped() {
        let now = Utc::now();
        let c = cfg();
        let cl8y = cw20(4, "CL8Y", 6, crate::config::DEFAULT_HUB_CL8Y_ADDRESS);
        let onyx = cw20(
            8,
            "NEON",
            6,
            "terra1listedeconomicxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        let only = pair(
            50,
            "terra1circ",
            cl8y,
            onyx,
            "10000000000",
            "10000000000",
            now,
        );
        let snap = snap_with_hub(now, &[only.clone()]);
        let marks = resolve_economic_marks(now, &c, &[only], &snap);
        assert!(marks.is_empty());
    }

    #[test]
    fn stale_and_zero_reserves_skipped() {
        let now = Utc::now();
        let c = cfg();
        let cl8y = cw20(4, "CL8Y", 6, crate::config::DEFAULT_HUB_CL8Y_ADDRESS);
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let old = now - chrono::Duration::seconds(120);
        let stale = pair(
            51,
            "terra1stalecl8y",
            ust1.clone(),
            cl8y.clone(),
            "100000000",
            "10000000000",
            old,
        );
        let empty = pair(52, "terra1emptycl8y", ust1, cl8y, "0", "10000000000", now);
        let snap = snap_with_hub(now, &[stale.clone(), empty.clone()]);
        let marks = resolve_economic_marks(now, &c, &[stale, empty], &snap);
        assert!(marks.is_empty());
    }

    #[test]
    fn other_listed_economic_not_cl8y_only() {
        let now = Utc::now();
        let c = cfg();
        let neon = cw20(
            5,
            "NEON",
            6,
            "terra1listedeconomicxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let p = pair(
            60,
            "terra1neonust1",
            ust1,
            neon,
            "100000000",
            "10000000000",
            now,
        );
        let snap = snap_with_hub(now, &[p.clone()]);
        let marks = resolve_economic_marks(now, &c, &[p], &snap);
        assert_eq!(marks.len(), 1);
        assert_eq!(
            marks[0].contract_address,
            "terra1listedeconomicxxxxxxxxxxxxxxxxxxxxxxxxxx"
        );
        assert!((usd_f(&marks[0].price_usd) - 0.01).abs() < 1e-9);
    }
}
