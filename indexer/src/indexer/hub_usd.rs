//! DEX hub USD marks from largest-liquidity factory pools (GitLab #556).
//!
//! Bootstrap (no circular quotes):
//! 1. `usd(cUSTC)` = `usd(uusd)` = #515 USTC CEX oracle (wrap 1:1). Oracle down → no hub USD.
//! 2. `usd(UST1)` from the max USD-TVL factory pair whose legs are hub cUSTC + hub UST1,
//!    using the constant-product **reserve** spot (not last swap print).
//! 3. `usd(USTR)` from the max USD-TVL factory pair vs already-priced cUSTC or UST1.
//!
//! Ranking is **humanized USD TVL** (sum of both legs), not raw reserve integers.
//! Ties: highest TVL, then lexicographic pair contract address.
//! Dust below `tvl_floor` (default $100), stale/missing/`reserve_*=0` rows, same-asset
//! legs, and symbol-spoof natives are skipped. Advisory only — not settlement.

use std::time::Duration;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};

use super::pair_price_usd::{fits_numeric_38_18, humanize_raw_amount};
use crate::config::Config;

/// Allowlisted hub-price path tickers (`GET /api/v1/hub-prices/{ticker}`).
pub const HUB_TICKERS: [&str; 3] = ["custc", "ust1", "ustr"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HubTicker {
    Custc,
    Ust1,
    Ustr,
}

impl HubTicker {
    pub fn as_str(self) -> &'static str {
        match self {
            HubTicker::Custc => "custc",
            HubTicker::Ust1 => "ust1",
            HubTicker::Ustr => "ustr",
        }
    }

    /// ASCII case-insensitive allowlist. Homoglyphs / `../` / extra underscores → None.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "custc" => Some(HubTicker::Custc),
            "ust1" => Some(HubTicker::Ust1),
            "ustr" => Some(HubTicker::Ustr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HubUsdConfig {
    pub custc_address: String,
    pub ust1_address: String,
    pub ustr_address: String,
    pub tvl_floor: BigDecimal,
    pub max_staleness: Duration,
}

impl HubUsdConfig {
    pub fn from_indexer_config(config: &Config) -> Self {
        Self {
            custc_address: config.hub_custc_address.clone(),
            ust1_address: config.hub_ust1_address.clone(),
            ustr_address: config.hub_ustr_address.clone(),
            tvl_floor: config.hub_usd_tvl_floor.clone(),
            max_staleness: Duration::from_millis(config.book_snapshot_max_staleness_ms()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssetRef {
    pub id: i32,
    #[allow(dead_code)]
    pub symbol: String,
    pub denom: Option<String>,
    pub contract_address: Option<String>,
    pub is_cw20: bool,
    pub decimals: i16,
}

#[derive(Debug, Clone)]
pub struct ReservePair {
    pub pair_id: i32,
    pub pair_address: String,
    pub asset_0: AssetRef,
    pub asset_1: AssetRef,
    pub reserve_0: BigDecimal,
    pub reserve_1: BigDecimal,
    pub snapshot_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HubMark {
    pub ticker: HubTicker,
    pub asset_id: Option<i32>,
    pub price_usd: BigDecimal,
    pub source_pair_id: Option<i32>,
    pub source_pair_address: Option<String>,
    pub tvl_usd: Option<BigDecimal>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HubUsdSnapshot {
    pub custc: Option<HubMark>,
    pub ust1: Option<HubMark>,
    pub ustr: Option<HubMark>,
}

fn addr_eq(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Hub cUSTC = configured CW20 **or** native `uusd` (1:1 wrap / USTC). Not symbol match.
pub fn is_hub_custc(asset: &AssetRef, cfg: &HubUsdConfig) -> bool {
    if let Some(d) = asset.denom.as_deref() {
        if d == "uusd" {
            return true;
        }
    }
    asset.is_cw20
        && asset
            .contract_address
            .as_deref()
            .is_some_and(|a| addr_eq(a, &cfg.custc_address))
}

pub fn is_hub_ust1(asset: &AssetRef, cfg: &HubUsdConfig) -> bool {
    asset.is_cw20
        && asset
            .contract_address
            .as_deref()
            .is_some_and(|a| addr_eq(a, &cfg.ust1_address))
}

pub fn is_hub_ustr(asset: &AssetRef, cfg: &HubUsdConfig) -> bool {
    asset.is_cw20
        && asset
            .contract_address
            .as_deref()
            .is_some_and(|a| addr_eq(a, &cfg.ustr_address))
}

fn same_asset(a: &AssetRef, b: &AssetRef) -> bool {
    a.id == b.id
}

fn reserves_usable(pair: &ReservePair) -> bool {
    pair.reserve_0 > BigDecimal::from(0) && pair.reserve_1 > BigDecimal::from(0)
}

fn is_stale(now: DateTime<Utc>, snapshot_at: DateTime<Utc>, max: Duration) -> bool {
    let age = now.signed_duration_since(snapshot_at);
    age.num_milliseconds() > max.as_millis() as i64
}

/// USD of `token` implied by CPAMM reserves vs an already-priced `other` leg.
pub fn usd_from_reserves(
    token_decimals: i16,
    other_decimals: i16,
    reserve_token: &BigDecimal,
    reserve_other: &BigDecimal,
    other_usd: &BigDecimal,
) -> Option<BigDecimal> {
    if other_usd <= &BigDecimal::from(0) {
        return None;
    }
    let h_token = humanize_raw_amount(reserve_token, token_decimals)?;
    let h_other = humanize_raw_amount(reserve_other, other_decimals)?;
    let usd = other_usd * h_other / h_token;
    if usd <= BigDecimal::from(0) || !fits_numeric_38_18(&usd) {
        None
    } else {
        Some(usd)
    }
}

fn pair_tvl(pair: &ReservePair, usd_0: &BigDecimal, usd_1: &BigDecimal) -> Option<BigDecimal> {
    let h0 = humanize_raw_amount(&pair.reserve_0, pair.asset_0.decimals)?;
    let h1 = humanize_raw_amount(&pair.reserve_1, pair.asset_1.decimals)?;
    let tvl = h0 * usd_0 + h1 * usd_1;
    if tvl <= BigDecimal::from(0) || !fits_numeric_38_18(&tvl) {
        None
    } else {
        Some(tvl)
    }
}

#[derive(Clone)]
struct Candidate {
    pair_id: i32,
    pair_address: String,
    price_usd: BigDecimal,
    tvl_usd: BigDecimal,
    asset_id: i32,
}

fn pick_best(mut cands: Vec<Candidate>) -> Option<Candidate> {
    cands.sort_by(|a, b| match b.tvl_usd.cmp(&a.tvl_usd) {
        std::cmp::Ordering::Equal => a.pair_address.cmp(&b.pair_address),
        o => o,
    });
    cands.into_iter().next()
}

fn candidate_for_token(
    pair: &ReservePair,
    token_is_0: bool,
    token_usd: &BigDecimal,
    other_usd: &BigDecimal,
    tvl_floor: &BigDecimal,
    token_id: i32,
) -> Option<Candidate> {
    let tvl = pair_tvl(
        pair,
        if token_is_0 { token_usd } else { other_usd },
        if token_is_0 { other_usd } else { token_usd },
    )?;
    if &tvl < tvl_floor {
        return None;
    }
    Some(Candidate {
        pair_id: pair.pair_id,
        pair_address: pair.pair_address.clone(),
        price_usd: token_usd.clone(),
        tvl_usd: tvl,
        asset_id: token_id,
    })
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

fn resolve_token_vs_priced_hub(
    now: DateTime<Utc>,
    cfg: &HubUsdConfig,
    pairs: &[ReservePair],
    is_token: impl Fn(&AssetRef) -> bool,
    is_priced_hub: impl Fn(&AssetRef) -> bool,
    hub_usd: impl Fn(&AssetRef) -> Option<BigDecimal>,
    ticker: HubTicker,
) -> Option<HubMark> {
    let mut cands = Vec::new();
    for pair in pairs {
        if !usable_pair(now, cfg, pair) {
            continue;
        }
        let (token, other, r_token, r_other, token_is_0) =
            if is_token(&pair.asset_0) && is_priced_hub(&pair.asset_1) {
                (
                    &pair.asset_0,
                    &pair.asset_1,
                    &pair.reserve_0,
                    &pair.reserve_1,
                    true,
                )
            } else if is_token(&pair.asset_1) && is_priced_hub(&pair.asset_0) {
                (
                    &pair.asset_1,
                    &pair.asset_0,
                    &pair.reserve_1,
                    &pair.reserve_0,
                    false,
                )
            } else {
                continue;
            };
        let other_usd = hub_usd(other)?;
        let token_usd =
            usd_from_reserves(token.decimals, other.decimals, r_token, r_other, &other_usd)?;
        if let Some(c) = candidate_for_token(
            pair,
            token_is_0,
            &token_usd,
            &other_usd,
            &cfg.tvl_floor,
            token.id,
        ) {
            cands.push(c);
        }
    }
    let best = pick_best(cands)?;
    Some(HubMark {
        ticker,
        asset_id: Some(best.asset_id),
        price_usd: best.price_usd,
        source_pair_id: Some(best.pair_id),
        source_pair_address: Some(best.pair_address),
        tvl_usd: Some(best.tvl_usd),
    })
}

/// Resolve cUSTC → UST1 → USTR from indexed factory pairs + reserves (H1–H3, H8).
pub fn resolve_hub_usd(
    now: DateTime<Utc>,
    cfg: &HubUsdConfig,
    ustc_oracle: Option<&BigDecimal>,
    pairs: &[ReservePair],
    custc_asset_id: Option<i32>,
) -> HubUsdSnapshot {
    let mut snap = HubUsdSnapshot::default();

    let Some(ustc) = ustc_oracle.filter(|p| **p > BigDecimal::from(0) && fits_numeric_38_18(p))
    else {
        return snap;
    };

    snap.custc = Some(HubMark {
        ticker: HubTicker::Custc,
        asset_id: custc_asset_id,
        price_usd: ustc.clone(),
        source_pair_id: None,
        source_pair_address: None,
        tvl_usd: None,
    });

    snap.ust1 = resolve_token_vs_priced_hub(
        now,
        cfg,
        pairs,
        |a| is_hub_ust1(a, cfg),
        |a| is_hub_custc(a, cfg),
        |_| Some(ustc.clone()),
        HubTicker::Ust1,
    );

    let ust1_usd = snap.ust1.as_ref().map(|m| m.price_usd.clone());
    snap.ustr = resolve_token_vs_priced_hub(
        now,
        cfg,
        pairs,
        |a| is_hub_ustr(a, cfg),
        |a| is_hub_custc(a, cfg) || (is_hub_ust1(a, cfg) && ust1_usd.is_some()),
        |a| {
            if is_hub_custc(a, cfg) {
                Some(ustc.clone())
            } else if is_hub_ust1(a, cfg) {
                ust1_usd.clone()
            } else {
                None
            }
        },
        HubTicker::Ustr,
    );

    snap
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    fn cfg() -> HubUsdConfig {
        HubUsdConfig {
            custc_address: "terra1custc".into(),
            ust1_address: "terra1ust1".into(),
            ustr_address: "terra1ustr".into(),
            tvl_floor: bd("100"),
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

    #[test]
    fn ticker_allowlist_rejects_injection() {
        assert_eq!(HubTicker::parse("ustr"), Some(HubTicker::Ustr));
        assert_eq!(HubTicker::parse("UST1"), Some(HubTicker::Ust1));
        assert!(HubTicker::parse("javascript:alert(1)").is_none());
        assert!(HubTicker::parse("../ustr").is_none());
        assert!(HubTicker::parse("ustc").is_none());
        assert!(HubTicker::parse("ustr_").is_none());
        assert!(HubTicker::parse("fdusd").is_none());
        assert!(HubTicker::parse("vfdusd").is_none());
    }

    #[test]
    fn h1_custc_tracks_oracle_missing_is_null() {
        let now = Utc::now();
        let c = cfg();
        let empty = resolve_hub_usd(now, &c, None, &[], Some(2));
        assert!(empty.custc.is_none());
        assert!(empty.ust1.is_none());
        assert!(empty.ustr.is_none());

        let snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &[], Some(2));
        assert_eq!(snap.custc.as_ref().unwrap().price_usd, bd("0.005"));
        assert_eq!(snap.custc.as_ref().unwrap().asset_id, Some(2));
        assert!(snap.ust1.is_none());
    }

    #[test]
    fn symbol_spoof_native_ustr_is_not_hub() {
        let c = cfg();
        let spoof = native(9, "USTR", "ugem", 18);
        assert!(!is_hub_ustr(&spoof, &c));
        let clone = cw20(8, "USTR", 18, "terra1clone");
        assert!(!is_hub_ustr(&clone, &c));
        let real = cw20(3, "GEM", 18, "terra1ustr");
        assert!(is_hub_ustr(&real, &c));
    }

    #[test]
    fn h2_ust1_from_deepest_custc_pool_not_peg1() {
        let now = Utc::now();
        let c = cfg();
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        // Thin: 10 human each → TVL ≈ 2 * 10 * 0.005 = $0.10 (below $100 floor)
        let thin = pair(
            10,
            "terra1thin",
            ust1.clone(),
            custc.clone(),
            "10000000",
            "10000000",
            now,
        );
        // Deep: 200 cUSTC per UST1; 50_000 human cUSTC + 250 human UST1
        // usd(UST1) = 0.005 * 50000 / 250 = 1.0; TVL = 250*1 + 50000*0.005 = $500
        let deep = pair(
            11,
            "terra1deep",
            ust1.clone(),
            custc.clone(),
            "250000000",
            "50000000000",
            now,
        );
        let snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &[thin, deep], Some(2));
        let u = snap.ust1.expect("ust1");
        assert!((usd_f(&u.price_usd) - 1.0).abs() < 1e-9);
        assert_eq!(u.source_pair_address.as_deref(), Some("terra1deep"));
        assert!((usd_f(u.tvl_usd.as_ref().unwrap()) - 500.0).abs() < 1e-6);
    }

    #[test]
    fn ust1_orientation_asset1_is_ust1() {
        let now = Utc::now();
        let c = cfg();
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        // asset_0 = cUSTC 50_000 human, asset_1 = UST1 250 human → same as H2 flipped
        let p = pair(
            12,
            "terra1flip",
            custc,
            ust1,
            "50000000000",
            "250000000",
            now,
        );
        let snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &[p], Some(2));
        let u = snap.ust1.expect("ust1");
        assert!((usd_f(&u.price_usd) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn raw_18_dec_cannot_beat_humanized_tvl() {
        let now = Utc::now();
        let c = cfg();
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ustr = cw20(3, "USTR", 18, "terra1ustr");
        // UST1 mark from a deep cUSTC pool so USTR can price vs UST1
        let ust1_pool = pair(
            11,
            "terra1deep",
            ust1.clone(),
            custc.clone(),
            "250000000",
            "50000000000",
            now,
        );
        // Huge raw 18-dec USTR (1e20 raw = 100 human) vs 1 human UST1 → thin USD TVL
        // usd(USTR) = usd(UST1)*1 / 100 = 0.01; TVL = 100*0.01 + 1*1 = $2 < floor
        let thin_raw = pair(
            20,
            "terra1wei",
            ust1.clone(),
            ustr.clone(),
            "1000000",
            "100000000000000000000",
            now,
        );
        // Real cUSTC/USTR: 20_000 human cUSTC + 10_000 human USTR (18-dec)
        // usd(USTR) = 0.005 * 20000 / 10000 = 0.01; TVL = 10000*0.01 + 20000*0.005 = $200
        let deep_custc = pair(
            21,
            "terra1ustrcustc",
            ustr,
            custc,
            "10000000000000000000000",
            "20000000000",
            now,
        );
        let snap = resolve_hub_usd(
            now,
            &c,
            Some(&bd("0.005")),
            &[ust1_pool, thin_raw, deep_custc],
            Some(2),
        );
        let u = snap.ustr.expect("ustr");
        assert_eq!(u.source_pair_address.as_deref(), Some("terra1ustrcustc"));
        assert!((usd_f(&u.price_usd) - 0.01).abs() < 1e-9);
    }

    #[test]
    fn h3_ustr_max_tvl_wins_not_2_5x_peg() {
        let now = Utc::now();
        let c = cfg();
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ustr = cw20(3, "USTR", 18, "terra1ustr");
        let ust1_pool = pair(
            11,
            "terra1deep",
            ust1.clone(),
            custc.clone(),
            "250000000",
            "50000000000",
            now,
        );
        // UST1/USTR: 100 USTR per UST1; 10_000 human USTR + 100 human UST1
        // usd(USTR) = 1.0 * 100 / 10000 = 0.01; TVL = 10000*0.01 + 100*1 = $200
        let vs_ust1 = pair(
            30,
            "terra1ustrust1",
            ust1,
            ustr.clone(),
            "100000000",
            "10000000000000000000000",
            now,
        );
        // Thinner cUSTC/USTR: TVL $100.10-ish just above floor but below $200
        let vs_custc = pair(
            31,
            "terra1thinustr",
            ustr,
            custc,
            "5000000000000000000000",
            "10000000000",
            now,
        );
        let snap = resolve_hub_usd(
            now,
            &c,
            Some(&bd("0.005")),
            &[ust1_pool, vs_ust1, vs_custc],
            Some(2),
        );
        let u = snap.ustr.expect("ustr");
        assert_eq!(u.source_pair_address.as_deref(), Some("terra1ustrust1"));
        assert!((usd_f(&u.price_usd) - 0.01).abs() < 1e-9);
        // Must not be 2.5 × USTC = 0.0125
        assert!((usd_f(&u.price_usd) - 0.0125).abs() > 0.001);
    }

    #[test]
    fn tie_lexicographic_pair_address() {
        let now = Utc::now();
        let c = cfg();
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let a = pair(
            1,
            "terra1bbb",
            ust1.clone(),
            custc.clone(),
            "250000000",
            "50000000000",
            now,
        );
        let b = pair(2, "terra1aaa", ust1, custc, "250000000", "50000000000", now);
        let snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &[a, b], Some(2));
        assert_eq!(
            snap.ust1.unwrap().source_pair_address.as_deref(),
            Some("terra1aaa")
        );
        let snap2 = resolve_hub_usd(
            now,
            &c,
            Some(&bd("0.005")),
            &[
                pair(
                    2,
                    "terra1aaa",
                    cw20(1, "UST1", 6, "terra1ust1"),
                    cw20(2, "cUSTC", 6, "terra1custc"),
                    "250000000",
                    "50000000000",
                    now,
                ),
                pair(
                    1,
                    "terra1bbb",
                    cw20(1, "UST1", 6, "terra1ust1"),
                    cw20(2, "cUSTC", 6, "terra1custc"),
                    "250000000",
                    "50000000000",
                    now,
                ),
            ],
            Some(2),
        );
        assert_eq!(
            snap2.ust1.unwrap().source_pair_address.as_deref(),
            Some("terra1aaa")
        );
    }

    #[test]
    fn dust_floor_skips_all_null() {
        let now = Utc::now();
        let c = cfg();
        let p = pair(
            1,
            "terra1dust",
            cw20(1, "UST1", 6, "terra1ust1"),
            cw20(2, "cUSTC", 6, "terra1custc"),
            "1000000",
            "1000000",
            now,
        );
        let snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &[p], Some(2));
        assert!(snap.ust1.is_none());
    }

    #[test]
    fn stale_reserves_skipped() {
        let now = Utc::now();
        let c = cfg();
        let old = now - chrono::Duration::seconds(120);
        let p = pair(
            1,
            "terra1stale",
            cw20(1, "UST1", 6, "terra1ust1"),
            cw20(2, "cUSTC", 6, "terra1custc"),
            "250000000",
            "50000000000",
            old,
        );
        let snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &[p], Some(2));
        assert!(snap.ust1.is_none());
    }

    #[test]
    fn zero_reserves_skipped() {
        let now = Utc::now();
        let c = cfg();
        let p = pair(
            1,
            "terra1empty",
            cw20(1, "UST1", 6, "terra1ust1"),
            cw20(2, "cUSTC", 6, "terra1custc"),
            "0",
            "50000000000",
            now,
        );
        let snap = resolve_hub_usd(now, &c, Some(&bd("0.005")), &[p], Some(2));
        assert!(snap.ust1.is_none());
    }

    #[test]
    fn regression_ustr_from_ust1_pool_ratio() {
        // USTR/UST1 at 100 USTR per UST1; UST1 = 200 cUSTC; usd(USTR) = usd(UST1)/100
        let now = Utc::now();
        let c = cfg();
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ustr = cw20(3, "USTR", 18, "terra1ustr");
        let ust1_pool = pair(
            11,
            "terra1deep",
            ust1.clone(),
            custc,
            "250000000",
            "50000000000",
            now,
        );
        let ustr_pool = pair(
            30,
            "terra1ustrust1",
            ust1,
            ustr,
            "100000000",
            "10000000000000000000000",
            now,
        );
        let snap = resolve_hub_usd(
            now,
            &c,
            Some(&bd("0.005")),
            &[ust1_pool, ustr_pool],
            Some(2),
        );
        let ust1_usd = usd_f(&snap.ust1.unwrap().price_usd);
        let ustr_usd = usd_f(&snap.ustr.unwrap().price_usd);
        assert!((ust1_usd - 1.0).abs() < 1e-9);
        assert!((ustr_usd - ust1_usd / 100.0).abs() < 1e-9);
    }
}
