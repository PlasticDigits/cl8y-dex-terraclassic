//! Protocol-wide factory pool TVL (GitLab #569).
//!
//! Sum of humanized USD of `pair_reserves` using the P522-Q catalog + hub marks.
//! One-sided catalog → `2×` the priced leg (CPAMM). Unpriced / stale / same-asset
//! / overflow pairs are omitted (not `$0`). Book escrow is not included.
//!
//! Computation runs on the aggregator / hub refresh — **not** on GET `/overview`
//! or GET `/pairs`. The same pass stamps `pair_liquidity_usd` for single-pair GET
//! (#664) and the `/pool` list JOIN (#655). Unpriced pairs have no row.

use std::time::Duration;

use bigdecimal::BigDecimal;
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};

use super::hub_usd::{is_stale, reserves_usable, same_asset, AssetRef, ReservePair};
use super::pair_price_usd::{
    fits_numeric_38_18, humanize_raw_amount, usd_per_human_quote, HubQuoteUsd, QuoteUsdKind,
};
use crate::db::queries::liquidity_snapshots::{LiquidityQuality, LiquidityRollup};
use crate::db::queries::pair_liquidity_usd::PairLiquidityStamp;
use crate::db::queries::{hub_prices, liquidity_snapshots};

/// Default book-mirror staleness when env is unset (cadence 10s × 2).
const DEFAULT_MAX_STALENESS: Duration = Duration::from_secs(20);

/// Freshness proof for a complete empty factory enumeration, independent of solver TTL.
pub const FACTORY_INVENTORY_MAX_AGE_SECONDS: i64 = 300;

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
    pub quality: LiquidityQuality,
}

impl ProtocolTvlSum {
    pub fn canonical_total(&self) -> Option<BigDecimal> {
        matches!(
            self.quality.status.as_str(),
            "fresh" | "partial" | "verified_empty"
        )
        .then(|| self.total_liquidity_usd.clone())
    }
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

/// TVL identity scope is fixed independently of quote availability. Metadata cannot
/// create a price handle, and a missing quote cannot change a two-leg pool to 2×.
fn catalog_kind(asset: &AssetRef) -> Option<QuoteUsdKind> {
    use crate::config::*;
    if !asset.is_cw20 {
        if asset.contract_address.is_some() {
            return None;
        }
        return match asset.denom.as_deref() {
            Some("uusd") => Some(QuoteUsdKind::Ustc),
            Some("uluna") => Some(QuoteUsdKind::Lunc),
            _ => None,
        };
    }
    if asset.denom.is_some() {
        return None;
    }
    let address = asset.contract_address.as_deref()?;
    for (name, default, kind) in [
        (
            "HUB_CUSTC_ADDRESS",
            DEFAULT_HUB_CUSTC_ADDRESS,
            QuoteUsdKind::Ustc,
        ),
        (
            "HUB_CLUNC_ADDRESS",
            DEFAULT_HUB_CLUNC_ADDRESS,
            QuoteUsdKind::Lunc,
        ),
        (
            "HUB_UST1_ADDRESS",
            DEFAULT_HUB_UST1_ADDRESS,
            QuoteUsdKind::Peg1,
        ),
        (
            "HUB_USTR_ADDRESS",
            DEFAULT_HUB_USTR_ADDRESS,
            QuoteUsdKind::Ustr,
        ),
        (
            "USDT_CW20_ADDRESS",
            DEFAULT_USDT_CW20_ADDRESS,
            QuoteUsdKind::Usdt,
        ),
    ] {
        let pin = std::env::var(name)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| default.to_string());
        if address == pin {
            return Some(kind);
        }
    }
    None
}

fn catalog_usd(asset: &AssetRef, quotes: &ProtocolTvlQuotes) -> Option<BigDecimal> {
    usd_per_human_quote(
        catalog_kind(asset)?,
        quotes.ustc.as_ref(),
        quotes.lunc.as_ref(),
        Some(&quotes.hub),
    )
    .filter(|p| *p > BigDecimal::from(0) && fits_numeric_38_18(p))
}

fn human_reserve(raw: &BigDecimal, decimals: i16) -> Option<BigDecimal> {
    if !(0..=38).contains(&decimals)
        || raw < &BigDecimal::from(0)
        || !raw.is_integer()
        || raw >= &crate::indexer::pair_price_usd::ten_pow_i32(38)
    {
        return None;
    }
    if raw == &BigDecimal::from(0) {
        return Some(BigDecimal::from(0));
    }
    humanize_raw_amount(raw, decimals)
}

/// TVL of a supported factory pair. A zero is known only from fresh, validated
/// reserves. Both supported legs require quotes; 2× applies only to one-sided catalog scope.
pub fn protocol_pair_tvl(pair: &ReservePair, quotes: &ProtocolTvlQuotes) -> Option<BigDecimal> {
    let h0 = human_reserve(&pair.reserve_0, pair.asset_0.decimals)?;
    let h1 = human_reserve(&pair.reserve_1, pair.asset_1.decimals)?;
    let k0 = catalog_kind(&pair.asset_0);
    let k1 = catalog_kind(&pair.asset_1);
    if k0.is_none() && k1.is_none() {
        return None;
    }
    // A completely drained pool has a known zero even during a price outage.
    if h0 == BigDecimal::from(0) && h1 == BigDecimal::from(0) {
        return Some(BigDecimal::from(0));
    }
    // Without a positive CPAMM ratio, 2× cannot value an unsupported residual leg.
    if (k0.is_none() || k1.is_none()) && (h0 == BigDecimal::from(0) || h1 == BigDecimal::from(0)) {
        return None;
    }
    let tvl = match (k0, k1) {
        (Some(_), Some(_)) => {
            h0 * catalog_usd(&pair.asset_0, quotes)? + h1 * catalog_usd(&pair.asset_1, quotes)?
        }
        (Some(_), None) => h0 * catalog_usd(&pair.asset_0, quotes)? * BigDecimal::from(2),
        (None, Some(_)) => h1 * catalog_usd(&pair.asset_1, quotes)? * BigDecimal::from(2),
        (None, None) => return None,
    };
    if tvl > BigDecimal::from(0) && tvl < crate::indexer::pair_price_usd::ten_pow_i32(-18) {
        return None; // A positive valuation must never round to a known zero in storage.
    }
    let tvl = tvl.round(18);
    (tvl >= BigDecimal::from(0) && fits_numeric_38_18(&tvl)).then_some(tvl)
}

pub fn pair_usable_for_protocol_tvl(
    now: chrono::DateTime<Utc>,
    max_staleness: Duration,
    pair: &ReservePair,
) -> bool {
    if pair.snapshot_at > now || same_asset(&pair.asset_0, &pair.asset_1) {
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

/// Expected identity membership includes supported pools with missing/stale inputs.
/// Hash contains full pair/asset identities and catalog modes; equal counts never
/// establish comparability. Unsupported-only pairs do not change this universe.
fn coverage_identity(pairs: &[ReservePair]) -> String {
    let mut members: Vec<_> = pairs
        .iter()
        .filter_map(|p| {
            let k0 = catalog_kind(&p.asset_0);
            let k1 = catalog_kind(&p.asset_1);
            if k0.is_none() && k1.is_none() {
                return None;
            }
            Some(
                serde_json::json!([
                    p.pair_id,
                    p.pair_address,
                    [
                        p.asset_0.id,
                        p.asset_0.is_cw20,
                        p.asset_0.denom,
                        p.asset_0.contract_address,
                        p.asset_0.decimals,
                        format!("{k0:?}")
                    ],
                    [
                        p.asset_1.id,
                        p.asset_1.is_cw20,
                        p.asset_1.denom,
                        p.asset_1.contract_address,
                        p.asset_1.decimals,
                        format!("{k1:?}")
                    ]
                ])
                .to_string(),
            )
        })
        .collect();
    members.sort();
    let mut digest = Sha256::new();
    for member in members {
        digest.update((member.len() as u64).to_be_bytes());
        digest.update(member.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

/// Full inventory census plus per-pair stamps. Omitted inputs are observations of
/// unknown quality, never a complete zero or a trusted percentage baseline.
pub fn collect_priced_pair_tvls(
    now: chrono::DateTime<Utc>,
    max_staleness: Duration,
    quotes: &ProtocolTvlQuotes,
    pairs: &[ReservePair],
) -> (ProtocolTvlSum, Vec<PairLiquidityStamp>) {
    let mut out = ProtocolTvlSum::default();
    let mut priced = Vec::new();
    let mut reasons = std::collections::BTreeMap::<&str, i32>::new();
    let mut expected = 0;
    let mut oldest = now;
    for pair in pairs {
        let supported =
            catalog_kind(&pair.asset_0).is_some() || catalog_kind(&pair.asset_1).is_some();
        let reason = if !supported {
            Some("unsupported_identity")
        } else {
            expected += 1;
            if pair.snapshot_at == chrono::DateTime::<Utc>::MIN_UTC {
                Some("missing_reserve")
            } else if pair.snapshot_at > now {
                Some("future_reserve")
            } else if is_stale(now, pair.snapshot_at, max_staleness) {
                Some("stale_reserve")
            } else if same_asset(&pair.asset_0, &pair.asset_1)
                || human_reserve(&pair.reserve_0, pair.asset_0.decimals).is_none()
                || human_reserve(&pair.reserve_1, pair.asset_1.decimals).is_none()
            {
                Some("invalid_reserve")
            } else {
                match protocol_pair_tvl(pair, quotes) {
                    Some(tvl) if fits_numeric_38_18(&(&out.total_liquidity_usd + &tvl)) => {
                        oldest = oldest.min(pair.snapshot_at);
                        priced.push(PairLiquidityStamp {
                            pair_id: pair.pair_id,
                            liquidity_usd: tvl.clone(),
                        });
                        out.total_liquidity_usd += tvl;
                        out.priced_pair_count += 1;
                        None
                    }
                    Some(_) => Some("overflow"),
                    None if catalog_kind(&pair.asset_0).is_some()
                        && catalog_usd(&pair.asset_0, quotes).is_none()
                        || catalog_kind(&pair.asset_1).is_some()
                            && catalog_usd(&pair.asset_1, quotes).is_none() =>
                    {
                        Some("missing_quote")
                    }
                    None => Some("invalid_valuation"),
                }
            }
        };
        if let Some(reason) = reason {
            *reasons.entry(reason).or_default() += 1;
            out.unpriced_pair_count += 1;
        }
    }
    let failed = expected - out.priced_pair_count;
    let status = if expected == 0 {
        if pairs.is_empty() {
            "verified_empty"
        } else {
            "unavailable"
        }
    } else if failed == 0 {
        "fresh"
    } else if out.priced_pair_count > 0 {
        "partial"
    } else if reasons.get("stale_reserve").copied().unwrap_or_default() == expected {
        "stale"
    } else {
        "unavailable"
    };
    out.quality = LiquidityQuality {
        status: status.into(),
        as_of: (out.priced_pair_count > 0 || pairs.is_empty()).then_some(oldest),
        coverage_identity: coverage_identity(pairs),
        generation: 0,
        reasons: serde_json::to_value(reasons).expect("bounded integer reason counts"),
    };
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

/// Quotes and reserves are read while input tables are stabilized. Expired hub/CEX
/// marks cannot make unchanged reserves look newly valued. No quote fallback/peg.
async fn load_quotes_tx(
    tx: &mut Transaction<'_, Postgres>,
    now: chrono::DateTime<Utc>,
    max_staleness: Duration,
) -> Result<
    (
        ProtocolTvlQuotes,
        std::collections::HashMap<&'static str, chrono::DateTime<Utc>>,
    ),
    sqlx::Error,
> {
    let oracle_age = crate::indexer::oracle::ORACLE_MAX_AGE_SECS as u64 * 1000;
    // Reserve-cache trust may last a full TTL after this valuation. Reserve a
    // source-lifetime budget so a nearly expired oracle cannot expire under it.
    let usable_age =
        oracle_age.saturating_sub(max_staleness.as_millis().min(u64::MAX as u128) as u64);
    let oracle_since = now - chrono::Duration::milliseconds(usable_age as i64);
    let rows: Vec<(String, BigDecimal)> = sqlx::query_as(
        "SELECT ticker, price_usd FROM oracle_state
         WHERE quality = 'fresh' AND cardinality(accepted_sources) >= 2
           AND price_usd BETWEEN 0.000000000001 AND 1000000
           AND as_of BETWEEN $1 AND $2",
    )
    .bind(oracle_since)
    .bind(now)
    .fetch_all(&mut **tx)
    .await?;
    let mut quotes = ProtocolTvlQuotes::default();
    for (ticker, price) in rows {
        match ticker.as_str() {
            "ustc" => quotes.ustc = Some(price),
            "lunc" => quotes.lunc = Some(price),
            _ => {}
        }
    }
    let hub_since =
        now - chrono::Duration::from_std(max_staleness).unwrap_or(chrono::Duration::seconds(20));
    let rows: Vec<(String, BigDecimal, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT ticker, price_usd, as_of FROM hub_prices WHERE as_of BETWEEN $1 AND $2
         AND quality = 'bounded_spot' AND price_usd > 0
         AND EXISTS (SELECT 1 FROM oracle_state WHERE ticker='ustc' AND quality='fresh'
             AND cardinality(accepted_sources)>=2 AND as_of BETWEEN $3 AND $2)",
    )
    .bind(hub_since)
    .bind(now)
    .bind(oracle_since)
    .fetch_all(&mut **tx)
    .await?;
    let mut hub_times = std::collections::HashMap::new();
    for (ticker, price, at) in rows {
        match ticker.as_str() {
            "ust1" => {
                quotes.hub.ust1 = Some(price);
                hub_times.insert("ust1", at);
            }
            "ustr" => {
                quotes.hub.ustr = Some(price);
                hub_times.insert("ustr", at);
            }
            _ => {}
        }
    }
    Ok((quotes, hub_times))
}

/// Both aggregator and hub callers lock before reading any inputs and publish one
/// generation transactionally. A crash rolls back the headline, stamps and history.
pub async fn refresh_protocol_liquidity(pool: &PgPool) -> Result<LiquidityRollup, sqlx::Error> {
    refresh_protocol_liquidity_with_staleness(pool, max_staleness_from_env()).await
}

pub async fn refresh_protocol_liquidity_with_staleness(
    pool: &PgPool,
    max_staleness: Duration,
) -> Result<LiquidityRollup, sqlx::Error> {
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(pool).await?;
    let now = Utc::now();
    let (quotes, hub_times) = load_quotes_tx(&mut tx, now, max_staleness).await?;
    let pairs = hub_prices::list_protocol_reserve_pairs_tx(&mut tx).await?;
    let previous = liquidity_snapshots::get_liquidity_rollup_tx(&mut tx).await?;
    let (mut sum, priced) = collect_priced_pair_tvls(now, max_staleness, &quotes, &pairs);
    if quotes.hub.ust1.is_some() || quotes.hub.ustr.is_some() {
        sum.quality.reasons["advisory_hub_spot"] = serde_json::json!(1);
    }
    // Hub marks have the same TTL as reserves: include their original time in
    // the oldest-used-input watermark instead of renewing a nearly expired mark.
    let priced_ids: std::collections::HashSet<_> =
        priced.iter().map(|stamp| stamp.pair_id).collect();
    for pair in &pairs {
        if !priced_ids.contains(&pair.pair_id)
            || (pair.reserve_0 == BigDecimal::from(0) && pair.reserve_1 == BigDecimal::from(0))
        {
            continue;
        }
        for asset in [&pair.asset_0, &pair.asset_1] {
            let ticker = match catalog_kind(asset) {
                Some(QuoteUsdKind::Peg1) => "ust1",
                Some(QuoteUsdKind::Ustr) => "ustr",
                _ => continue,
            };
            if let Some(at) = hub_times.get(ticker) {
                sum.quality.as_of = sum.quality.as_of.map(|current| current.min(*at));
            }
        }
    }
    // A failed first enumeration may leave a populated subset. Rows alone do
    // not establish expected factory membership, even when their reserves are fresh.
    let observed: Option<String> = sqlx::query_scalar(
        "SELECT value FROM indexer_state WHERE key = 'factory_inventory_observed_at'",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let observed = observed
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(&v).ok())
        .map(|t| t.with_timezone(&Utc))
        .filter(|t| *t <= now);
    let inventory_known = observed.is_some_and(|t| {
        !pairs.is_empty()
            || now.signed_duration_since(t)
                <= chrono::Duration::seconds(FACTORY_INVENTORY_MAX_AGE_SECONDS)
    });
    if !inventory_known {
        sum.quality.status = "unavailable".into();
        sum.quality.as_of = None;
        sum.quality.reasons["unknown_inventory"] = serde_json::json!(1);
    } else if pairs.is_empty() {
        sum.quality.as_of = observed;
    }
    sum.quality.generation = previous
        .quality
        .generation
        .checked_add(1)
        .ok_or_else(|| sqlx::Error::Protocol("liquidity generation exhausted".into()))?;
    let comparable = matches!(sum.quality.status.as_str(), "fresh" | "verified_empty");
    let then_24h = liquidity_snapshots::nearest_comparable_snapshot_tx(
        &mut tx,
        now - chrono::Duration::hours(24),
        &sum.quality.coverage_identity,
    )
    .await?;
    let then_30d = liquidity_snapshots::nearest_comparable_snapshot_tx(
        &mut tx,
        now - chrono::Duration::days(30),
        &sum.quality.coverage_identity,
    )
    .await?;
    let usd_24h = then_24h.and_then(|s| s.total_liquidity_usd);
    let usd_30d = then_30d.and_then(|s| s.total_liquidity_usd);
    let rollup = LiquidityRollup {
        liquidity_change_24h_pct: if comparable {
            usd_24h
                .as_ref()
                .and_then(|v| pct_change(&sum.total_liquidity_usd, v))
        } else {
            None
        },
        liquidity_change_30d_pct: if comparable {
            usd_30d
                .as_ref()
                .and_then(|v| pct_change(&sum.total_liquidity_usd, v))
        } else {
            None
        },
        total_liquidity_usd: sum.canonical_total(),
        priced_pair_count: sum.priced_pair_count,
        unpriced_pair_count: sum.unpriced_pair_count,
        total_liquidity_usd_24h_ago: if comparable { usd_24h } else { None },
        total_liquidity_usd_30d_ago: if comparable { usd_30d } else { None },
        quality: sum.quality,
    };
    tracing::info!(quality = %rollup.quality.status, previous_quality = %previous.quality.status,
        priced = rollup.priced_pair_count, unpriced = rollup.unpriced_pair_count,
        generation = rollup.quality.generation, reasons = %rollup.quality.reasons,
        as_of = ?rollup.quality.as_of, "Protocol liquidity coverage observation");
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &rollup, &priced, now).await?;
    tx.commit().await?;
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
            contract_address: Some(
                match symbol {
                    "UST1" => crate::config::DEFAULT_HUB_UST1_ADDRESS,
                    "USTR" => crate::config::DEFAULT_HUB_USTR_ADDRESS,
                    "cUSTC" => crate::config::DEFAULT_HUB_CUSTC_ADDRESS,
                    "cLUNC" => crate::config::DEFAULT_HUB_CLUNC_ADDRESS,
                    _ => contract,
                }
                .into(),
            ),
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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

    #[test]
    fn outage_recovery_and_real_changes_have_distinct_quality() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-04T06:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut p = pair(
            1,
            "terra1p",
            native(1, "USTC", "uusd", 6),
            native(2, "LUNC", "uluna", 6),
            "100000000",
            "100000000",
            now,
        );
        let q = quotes_ustc_lunc_hub();
        let good = sum_protocol_tvl(now, Duration::from_secs(20), &q, &[p.clone()]);
        assert_eq!(good.quality.status, "fresh");
        assert_eq!(good.canonical_total(), Some(bd("1.01")));
        let stale = sum_protocol_tvl(
            now + chrono::Duration::seconds(21),
            Duration::from_secs(20),
            &q,
            &[p.clone()],
        );
        assert_eq!(stale.quality.status, "stale");
        assert_eq!(stale.canonical_total(), None);
        assert_eq!(
            stale.quality.coverage_identity,
            good.quality.coverage_identity
        );
        let missing = sum_protocol_tvl(
            now,
            Duration::from_secs(20),
            &ProtocolTvlQuotes::default(),
            &[p.clone()],
        );
        assert_eq!(missing.quality.status, "unavailable");
        assert_eq!(missing.canonical_total(), None);
        assert_eq!(
            sum_protocol_tvl(now, Duration::from_secs(20), &q, &[p.clone()]).canonical_total(),
            good.canonical_total()
        );
        p.reserve_0 = bd("0");
        p.reserve_1 = bd("0");
        let drained = sum_protocol_tvl(
            now,
            Duration::from_secs(20),
            &ProtocolTvlQuotes::default(),
            &[p],
        );
        assert_eq!(drained.quality.status, "fresh");
        assert_eq!(drained.canonical_total(), Some(bd("0")));
        assert_eq!(pct_change(&bd("0"), &bd("1.01")), Some(bd("-100")));
    }

    #[test]
    fn material_pool_loss_is_partial_and_membership_is_not_a_count() {
        let now = Utc::now();
        let small = pair(
            1,
            "terra1small",
            native(1, "USTC", "uusd", 6),
            cw20(2, "GEM", 6, "terra1gem"),
            "1000000",
            "1",
            now,
        );
        let mut large = small.clone();
        large.pair_id = 2;
        large.pair_address = "terra1large".into();
        large.reserve_0 = bd("1000000000000");
        let q = quotes_ustc_lunc_hub();
        let good = sum_protocol_tvl(
            now,
            Duration::from_secs(20),
            &q,
            &[small.clone(), large.clone()],
        );
        large.snapshot_at = now - chrono::Duration::seconds(21);
        let partial = sum_protocol_tvl(
            now,
            Duration::from_secs(20),
            &q,
            &[small.clone(), large.clone()],
        );
        assert_eq!(partial.quality.status, "partial");
        assert_eq!(
            partial.quality.coverage_identity,
            good.quality.coverage_identity
        );
        assert_eq!(partial.canonical_total(), Some(bd("0.02")));
        large.pair_address = "terra1replacement".into();
        assert_ne!(
            sum_protocol_tvl(now, Duration::from_secs(20), &q, &[small, large])
                .quality
                .coverage_identity,
            good.quality.coverage_identity
        );
    }

    #[test]
    fn missing_future_invalid_and_overflow_are_observable() {
        let now = Utc::now();
        let mut p = pair(
            1,
            "terra1p",
            native(1, "USTC", "uusd", 6),
            cw20(2, "GEM", 6, "terra1gem"),
            "1000000",
            "1",
            now,
        );
        let q = quotes_ustc_lunc_hub();
        for (stamp, reason) in [
            (chrono::DateTime::<Utc>::MIN_UTC, "missing_reserve"),
            (now + chrono::Duration::seconds(1), "future_reserve"),
        ] {
            p.snapshot_at = stamp;
            let s = sum_protocol_tvl(now, Duration::from_secs(20), &q, &[p.clone()]);
            assert_eq!(s.canonical_total(), None);
            assert_eq!(s.quality.reasons[reason], 1);
        }
        p.snapshot_at = now;
        p.reserve_0 = bd("-1");
        let s = sum_protocol_tvl(now, Duration::from_secs(20), &q, &[p.clone()]);
        assert_eq!(s.quality.reasons["invalid_reserve"], 1);
        p.reserve_0 = bd("99999999999999999999999999999999999999");
        assert_eq!(
            sum_protocol_tvl(now, Duration::from_secs(20), &q, &[p])
                .quality
                .reasons["invalid_valuation"],
            1
        );
    }

    #[test]
    fn supported_leg_price_loss_never_switches_valuation_method() {
        let now = Utc::now();
        let p = pair(
            1,
            "terra1p",
            native(1, "USTC", "uusd", 6),
            cw20(2, "USTR", 18, "ignored"),
            "1000000",
            "1000000000000000000",
            now,
        );
        let mut q = quotes_ustc_lunc_hub();
        q.hub.ustr = None;
        assert_eq!(protocol_pair_tvl(&p, &q), None);
        assert_eq!(
            sum_protocol_tvl(now, Duration::from_secs(20), &q, &[p])
                .quality
                .reasons["missing_quote"],
            1
        );
    }

    #[test]
    fn unsupported_inventory_does_not_block_fresh_catalog_and_spoof_cw20_is_not_a_handle() {
        let now = Utc::now();
        let good = pair(
            1,
            "terra1p",
            native(1, "USTC", "uusd", 6),
            cw20(2, "GEM", 6, "terra1gem"),
            "1000000",
            "1",
            now,
        );
        let mut spoof = cw20(3, "GEM", 6, "terra1malicious");
        spoof.symbol = "UST1".into();
        let unsupported = pair(
            2,
            "terra1spoof",
            spoof,
            cw20(4, "GEM", 6, "terra1g"),
            "1000000",
            "1",
            chrono::DateTime::<Utc>::MIN_UTC,
        );
        let s = sum_protocol_tvl(
            now,
            Duration::from_secs(20),
            &quotes_ustc_lunc_hub(),
            &[good, unsupported],
        );
        assert_eq!(s.quality.status, "fresh");
        assert_eq!(s.quality.reasons["unsupported_identity"], 1);
        assert_eq!(s.canonical_total(), Some(bd("0.02")));
    }

    #[test]
    fn positive_underflow_and_one_sided_zero_are_unknown_not_verified_empty() {
        let now = Utc::now();
        let q = quotes_ustc_lunc_hub();
        let mut p = pair(
            1,
            "terra1dust",
            native(1, "USTC", "uusd", 38),
            cw20(2, "GEM", 6, "terra1gem"),
            "1",
            "1",
            now,
        );
        assert_eq!(protocol_pair_tvl(&p, &q), None);
        assert_eq!(
            sum_protocol_tvl(now, Duration::from_secs(20), &q, &[p.clone()]).canonical_total(),
            None
        );
        p.asset_0.decimals = 6;
        p.reserve_0 = bd("0");
        assert_eq!(protocol_pair_tvl(&p, &q), None);
        p.reserve_1 = bd("0");
        assert_eq!(protocol_pair_tvl(&p, &q), Some(bd("0")));
    }
}
