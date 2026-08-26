//! Protocol-wide factory pool TVL (GitLab #569).
//!
//! Sum of humanized USD of `pair_reserves` using the P522-Q catalog + hub marks.
//! One-sided catalog → `2×` the priced leg (CPAMM). Unpriced / stale / same-asset
//! / overflow pairs are omitted (not `$0`). Book escrow is not included.
//!
//! Computation runs on the aggregator / hub refresh — **not** on GET `/overview`.
//! The same pass stamps `pair_liquidity_usd` for single-pair GET (#664) and the
//! `/pool` list JOIN (#655).

use std::time::Duration;

use bigdecimal::BigDecimal;
use chrono::Utc;
use sqlx::PgPool;

use super::hub_usd::{AssetRef, ReservePair, is_stale, pair_tvl, reserves_usable, same_asset};
use super::oracle::OracleTicker;
use super::pair_price_usd::{
    HubQuoteUsd, catalog_usd_per_human_identity, fits_numeric_38_18, humanize_raw_amount,
};
use crate::db::queries::liquidity_snapshots::LiquidityRollup;
use crate::db::queries::pair_liquidity_usd::PairLiquidityStamp;
use crate::db::queries::{hub_prices, liquidity_snapshots, oracle, pair_liquidity_usd};

/// Default book-mirror staleness when env is unset (cadence 10s × 2).
const DEFAULT_MAX_STALENESS: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Default)]
pub struct ProtocolTvlQuotes {
    pub ustc: Option<BigDecimal>,
    pub lunc: Option<BigDecimal>,
    pub hub: HubQuoteUsd,
}

#[derive(Debug, Clone, Default)]
pub struct ProtocolTvlSum {
    pub total_liquidity_usd: BigDecimal,
    pub priced_pair_count: i32,
    pub unpriced_pair_count: i32,
}

/// `pct = (now − then) / then × 100` when `then > 0` and the result fits NUMERIC(38,18).
pub fn pct_change(now: &BigDecimal, then: &BigDecimal) -> Option<BigDecimal> {
    if then <= &BigDecimal::from(0) {
        return None;
    }
    let pct = (now - then) / then * BigDecimal::from(100);
    if !fits_numeric_38_18(&pct) {
        None
    } else {
        Some(pct)
    }
}

fn catalog_usd(asset: &AssetRef, quotes: &ProtocolTvlQuotes) -> Option<BigDecimal> {
    catalog_usd_per_human_identity(
        &asset.symbol,
        asset.denom.as_deref(),
        asset.is_cw20,
        asset.contract_address.as_deref(),
        quotes.ustc.as_ref(),
        quotes.lunc.as_ref(),
        None,
        Some(&quotes.hub),
    )
}

fn one_sided_tvl(human: &BigDecimal, usd: &BigDecimal) -> Option<BigDecimal> {
    let tvl = human * usd * BigDecimal::from(2);
    if tvl <= BigDecimal::from(0) || !fits_numeric_38_18(&tvl) {
        None
    } else {
        Some(tvl)
    }
}

/// TVL of one factory pair. `None` = omit from the protocol sum (not `$0`).
pub fn protocol_pair_tvl(pair: &ReservePair, quotes: &ProtocolTvlQuotes) -> Option<BigDecimal> {
    let usd_0 = catalog_usd(&pair.asset_0, quotes);
    let usd_1 = catalog_usd(&pair.asset_1, quotes);
    match (usd_0.as_ref(), usd_1.as_ref()) {
        (Some(u0), Some(u1)) => pair_tvl(pair, u0, u1),
        (Some(u0), None) => {
            let h0 = humanize_raw_amount(&pair.reserve_0, pair.asset_0.decimals)?;
            one_sided_tvl(&h0, u0)
        }
        (None, Some(u1)) => {
            let h1 = humanize_raw_amount(&pair.reserve_1, pair.asset_1.decimals)?;
            one_sided_tvl(&h1, u1)
        }
        (None, None) => None,
    }
}

pub fn pair_usable_for_protocol_tvl(
    now: chrono::DateTime<Utc>,
    max_staleness: Duration,
    pair: &ReservePair,
) -> bool {
    if same_asset(&pair.asset_0, &pair.asset_1) {
        return false;
    }
    if !reserves_usable(pair) {
        return false;
    }
    if is_stale(now, pair.snapshot_at, max_staleness) {
        return false;
    }
    true
}

/// Census sum plus per-pair stamps (unpriced pairs are absent, not `$0`).
pub fn collect_priced_pair_tvls(
    now: chrono::DateTime<Utc>,
    max_staleness: Duration,
    quotes: &ProtocolTvlQuotes,
    pairs: &[ReservePair],
) -> (ProtocolTvlSum, Vec<PairLiquidityStamp>) {
    let mut out = ProtocolTvlSum::default();
    let mut priced = Vec::new();
    for pair in pairs {
        if !pair_usable_for_protocol_tvl(now, max_staleness, pair) {
            out.unpriced_pair_count += 1;
            continue;
        }
        match protocol_pair_tvl(pair, quotes) {
            Some(tvl) => {
                priced.push(PairLiquidityStamp {
                    pair_id: pair.pair_id,
                    liquidity_usd: tvl.clone(),
                });
                out.total_liquidity_usd += tvl;
                out.priced_pair_count += 1;
            }
            None => out.unpriced_pair_count += 1,
        }
    }
    if !fits_numeric_38_18(&out.total_liquidity_usd) {
        // Clamp rather than wrap / Inf — GET stays 200.
        out.total_liquidity_usd = crate::indexer::pair_price_usd::ten_pow_i32(20)
            - crate::indexer::pair_price_usd::ten_pow_i32(-18);
    }
    (out, priced)
}

pub fn sum_protocol_tvl(
    now: chrono::DateTime<Utc>,
    max_staleness: Duration,
    quotes: &ProtocolTvlQuotes,
    pairs: &[ReservePair],
) -> ProtocolTvlSum {
    collect_priced_pair_tvls(now, max_staleness, quotes, pairs).0
}

pub fn max_staleness_from_env() -> Duration {
    std::env::var("BOOK_SNAPSHOT_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|ms| Duration::from_millis(ms.saturating_mul(2).max(1_000)))
        .unwrap_or(DEFAULT_MAX_STALENESS)
}

async fn load_quotes(pool: &PgPool) -> Result<ProtocolTvlQuotes, sqlx::Error> {
    let ustc = oracle::get_latest_average_price(pool, OracleTicker::Ustc).await?;
    let lunc = oracle::get_latest_average_price(pool, OracleTicker::Lunc).await?;
    let hub = hub_prices::load_quote_usd(pool).await?;
    Ok(ProtocolTvlQuotes { ustc, lunc, hub })
}

/// Recompute current TVL, Δ% vs snapshots, upsert rollup, insert/prune history.
pub async fn refresh_protocol_liquidity(pool: &PgPool) -> Result<LiquidityRollup, sqlx::Error> {
    refresh_protocol_liquidity_with_staleness(pool, max_staleness_from_env()).await
}

pub async fn refresh_protocol_liquidity_with_staleness(
    pool: &PgPool,
    max_staleness: Duration,
) -> Result<LiquidityRollup, sqlx::Error> {
    let now = Utc::now();
    let quotes = load_quotes(pool).await?;
    let pairs = hub_prices::list_reserve_pairs(pool).await?;
    let (sum, priced) = collect_priced_pair_tvls(now, max_staleness, &quotes, &pairs);

    let then_24h =
        liquidity_snapshots::nearest_snapshot(pool, now - chrono::Duration::hours(24)).await?;
    let then_30d =
        liquidity_snapshots::nearest_snapshot(pool, now - chrono::Duration::days(30)).await?;

    let usd_24h = then_24h.as_ref().map(|s| s.total_liquidity_usd.clone());
    let usd_30d = then_30d.as_ref().map(|s| s.total_liquidity_usd.clone());

    let rollup = LiquidityRollup {
        liquidity_change_24h_pct: usd_24h
            .as_ref()
            .and_then(|then| pct_change(&sum.total_liquidity_usd, then)),
        liquidity_change_30d_pct: usd_30d
            .as_ref()
            .and_then(|then| pct_change(&sum.total_liquidity_usd, then)),
        total_liquidity_usd: sum.total_liquidity_usd.clone(),
        priced_pair_count: sum.priced_pair_count,
        unpriced_pair_count: sum.unpriced_pair_count,
        total_liquidity_usd_24h_ago: usd_24h,
        total_liquidity_usd_30d_ago: usd_30d,
    };

    liquidity_snapshots::upsert_liquidity_rollup(pool, &rollup).await?;
    liquidity_snapshots::maybe_insert_snapshot(
        pool,
        now,
        &sum.total_liquidity_usd,
        sum.priced_pair_count,
    )
    .await?;
    liquidity_snapshots::prune_snapshots(pool, now).await?;
    pair_liquidity_usd::replace_pair_liquidity_usd(pool, &priced).await?;
    Ok(rollup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::hub_usd::{AssetRef, ReservePair};
    use chrono::{DateTime, Utc};
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
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

    fn quotes_ustc_lunc_hub() -> ProtocolTvlQuotes {
        ProtocolTvlQuotes {
            ustc: Some(bd("0.01")),
            lunc: Some(bd("0.0001")),
            hub: HubQuoteUsd {
                ust1: Some(bd("0.98")),
                ustr: Some(bd("0.012")),
            },
        }
    }

    #[test]
    fn t1_both_catalogued_legs_6_and_18_dec() {
        let now = Utc::now();
        let uusd = native(1, "USTC", "uusd", 6);
        let ustr = cw20(2, "USTR", 18, "terra1ustr");
        // 100 human USTC + 50 human USTR
        let p = pair(
            1,
            "terra1p",
            uusd,
            ustr,
            "100000000",
            "50000000000000000000",
            now,
        );
        let q = quotes_ustc_lunc_hub();
        let tvl = protocol_pair_tvl(&p, &q).expect("tvl");
        // 100*0.01 + 50*0.012 = 1 + 0.6 = 1.6
        assert_eq!(tvl, bd("1.6"));
    }

    #[test]
    fn t2_one_catalogued_leg_is_2x() {
        let now = Utc::now();
        let uusd = native(1, "USTC", "uusd", 6);
        let gem = cw20(2, "GEM", 18, "terra1gem");
        // 200 human USTC, gem unpriced
        let p = pair(
            1,
            "terra1p",
            uusd,
            gem,
            "200000000",
            "100000000000000000000",
            now,
        );
        let q = quotes_ustc_lunc_hub();
        let tvl = protocol_pair_tvl(&p, &q).expect("tvl");
        // 2 * 200 * 0.01 = 4
        assert_eq!(tvl, bd("4"));
    }

    #[test]
    fn t3_neither_catalogued_omitted() {
        let now = Utc::now();
        let g0 = cw20(1, "GEM", 6, "terra1g0");
        let g1 = cw20(2, "GEM2", 6, "terra1g1");
        let p = pair(1, "terra1p", g0, g1, "1000000", "1000000", now);
        assert!(protocol_pair_tvl(&p, &quotes_ustc_lunc_hub()).is_none());
        let sum = sum_protocol_tvl(now, Duration::from_secs(60), &quotes_ustc_lunc_hub(), &[p]);
        assert_eq!(sum.total_liquidity_usd, bd("0"));
        assert_eq!(sum.priced_pair_count, 0);
        assert_eq!(sum.unpriced_pair_count, 1);
    }

    #[test]
    fn t4_ust1_custc_uses_hub_not_peg1() {
        let now = Utc::now();
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        // 10 human UST1 + 20 human cUSTC
        let p = pair(1, "terra1p", ust1, custc, "10000000", "20000000", now);
        let q = quotes_ustc_lunc_hub();
        let tvl = protocol_pair_tvl(&p, &q).expect("tvl");
        // 10*0.98 + 20*0.01 = 9.8 + 0.2 = 10.0 (not 10*$1 + 20*0.01 = 10.2)
        assert_eq!(tvl, bd("10.0"));
        let pegged = ProtocolTvlQuotes {
            ustc: Some(bd("0.01")),
            lunc: None,
            hub: HubQuoteUsd {
                ust1: Some(bd("1")),
                ustr: None,
            },
        };
        let tvl_peg = protocol_pair_tvl(&p, &pegged).expect("tvl peg");
        assert_ne!(tvl, tvl_peg);
    }

    #[test]
    fn t5_ustr_vs_ust1_uses_hub_not_2_5x() {
        let now = Utc::now();
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ustr = cw20(2, "USTR", 18, "terra1ustr");
        // 10 human UST1 + 100 human USTR
        let p = pair(
            1,
            "terra1p",
            ust1,
            ustr,
            "10000000",
            "100000000000000000000",
            now,
        );
        let q = quotes_ustc_lunc_hub();
        let tvl = protocol_pair_tvl(&p, &q).expect("tvl");
        // 10*0.98 + 100*0.012 = 9.8 + 1.2 = 11.0
        // fake 2.5× USTC: usd(USTR)=0.025 → 10*0.98 + 100*0.025 = 12.3
        assert_eq!(tvl, bd("11.0"));
        assert_ne!(tvl, bd("12.3"));
    }

    #[test]
    fn t6_zero_missing_stale_reserves_omitted() {
        let now = Utc::now();
        let uusd = native(1, "USTC", "uusd", 6);
        let uluna = native(2, "LUNC", "uluna", 6);
        let zero = pair(
            1,
            "terra1z",
            uusd.clone(),
            uluna.clone(),
            "0",
            "1000000",
            now,
        );
        let q = quotes_ustc_lunc_hub();
        assert!(protocol_pair_tvl(&zero, &q).is_none() || !reserves_usable(&zero));
        assert!(!pair_usable_for_protocol_tvl(
            now,
            Duration::from_secs(60),
            &zero
        ));

        let stale = pair(
            2,
            "terra1s",
            uusd,
            uluna,
            "1000000",
            "1000000",
            now - chrono::Duration::seconds(120),
        );
        assert!(!pair_usable_for_protocol_tvl(
            now,
            Duration::from_secs(60),
            &stale
        ));
        let sum = sum_protocol_tvl(now, Duration::from_secs(60), &q, &[stale]);
        assert_eq!(sum.priced_pair_count, 0);
        assert_eq!(sum.total_liquidity_usd, bd("0"));
    }

    #[test]
    fn t7_same_asset_pair_omitted() {
        let now = Utc::now();
        let uusd = native(1, "USTC", "uusd", 6);
        let p = pair(1, "terra1p", uusd.clone(), uusd, "1000000", "1000000", now);
        assert!(!pair_usable_for_protocol_tvl(
            now,
            Duration::from_secs(60),
            &p
        ));
    }

    #[test]
    fn t9_overflow_huge_18_dec_skipped() {
        let now = Utc::now();
        let uusd = native(1, "USTC", "uusd", 6);
        let ustr = cw20(2, "USTR", 18, "terra1ustr");
        // 10^38 raw 18-dec is still human 10^20 — may fail fits_numeric after * usd
        let huge = pair(
            1,
            "terra1h",
            uusd,
            ustr,
            "1",
            "99999999999999999999999999999999999999",
            now,
        );
        let q = ProtocolTvlQuotes {
            ustc: Some(bd("1")),
            lunc: None,
            hub: HubQuoteUsd {
                ust1: None,
                ustr: Some(bd("1")),
            },
        };
        let tvl = protocol_pair_tvl(&huge, &q);
        if let Some(v) = tvl {
            assert!(fits_numeric_38_18(&v));
        }
        let sum = sum_protocol_tvl(now, Duration::from_secs(60), &q, &[huge]);
        assert!(fits_numeric_38_18(&sum.total_liquidity_usd));
    }

    #[test]
    fn t10_hub_down_omits_unpriced_hub_legs() {
        let now = Utc::now();
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let gem = cw20(2, "GEM", 6, "terra1gem");
        let p = pair(1, "terra1p", ust1, gem, "1000000", "1000000", now);
        let q = ProtocolTvlQuotes {
            ustc: Some(bd("0.01")),
            lunc: None,
            hub: HubQuoteUsd::default(),
        };
        assert!(protocol_pair_tvl(&p, &q).is_none());
    }

    #[test]
    fn a5_symbol_spoof_native_ustr_does_not_price() {
        let now = Utc::now();
        let spoof = native(9, "USTR", "ugem", 18);
        let gem = cw20(2, "GEM", 6, "terra1gem");
        let p = pair(
            1,
            "terra1p",
            spoof,
            gem,
            "1000000000000000000",
            "1000000",
            now,
        );
        assert!(protocol_pair_tvl(&p, &quotes_ustc_lunc_hub()).is_none());
    }

    #[test]
    fn a7_vfdusd_never_enters_tvl() {
        let now = Utc::now();
        let fd = cw20(1, "vFDUSD", 6, "terra1vfd");
        let gem = cw20(2, "GEM", 6, "terra1gem");
        let p = pair(1, "terra1p", fd, gem, "1000000", "1000000", now);
        let q = ProtocolTvlQuotes {
            ustc: Some(bd("0.01")),
            lunc: Some(bd("0.0001")),
            hub: HubQuoteUsd::default(),
        };
        assert!(protocol_pair_tvl(&p, &q).is_none());
    }

    #[test]
    fn a4_humanize_prevents_raw_18_dec_takeover() {
        let now = Utc::now();
        let uusd = native(1, "USTC", "uusd", 6);
        let gem = cw20(2, "GEM", 18, "terra1gem");
        // 1 human USTC vs huge raw 18-dec gem (unpriced) → one-sided 2× USTC only
        let p = pair(
            1,
            "terra1p",
            uusd,
            gem,
            "1000000",
            "1000000000000000000000000",
            now,
        );
        let tvl = protocol_pair_tvl(&p, &quotes_ustc_lunc_hub()).expect("tvl");
        // 2 * 1 * 0.01 = 0.02 — not dominated by 1e24 raw
        assert_eq!(tvl, bd("0.02"));
    }

    #[test]
    fn pct_null_when_then_zero_or_missing() {
        assert!(pct_change(&bd("10"), &bd("0")).is_none());
        assert_eq!(pct_change(&bd("150"), &bd("100")).unwrap(), bd("50"));
        assert_eq!(pct_change(&bd("50"), &bd("100")).unwrap(), bd("-50"));
    }

    #[test]
    fn double_count_across_pools_is_correct() {
        let now = Utc::now();
        let uusd = native(1, "USTC", "uusd", 6);
        let g0 = cw20(2, "GEM", 6, "terra1g0");
        let g1 = cw20(3, "GEM2", 6, "terra1g1");
        let a = pair(1, "terra1a", uusd.clone(), g0, "100000000", "1", now); // 2×100×0.01 = 2
        let b = pair(2, "terra1b", uusd, g1, "100000000", "1", now); // 2
        let sum = sum_protocol_tvl(
            now,
            Duration::from_secs(60),
            &quotes_ustc_lunc_hub(),
            &[a, b],
        );
        assert_eq!(sum.total_liquidity_usd, bd("4"));
        assert_eq!(sum.priced_pair_count, 2);
    }
}
