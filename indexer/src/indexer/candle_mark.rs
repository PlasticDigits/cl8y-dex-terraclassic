//! Idle mark-to-market USD candles (GitLab #568).
//!
//! Hub refresh must **not** rewrite historical `swap_events.price_usd` or candle USD.
//! After the live `hub_prices` snapshot is replaced, this writer upserts the **current**
//! interval buckets for catalog-quoted factory pairs: `usd = last_human × as-of quote USD`.
//! Mark bars keep `trade_count = 0` and zero volume. Advisory only — not settlement.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use super::candle_builder;
use super::hub_usd::{is_hub_custc, is_hub_ust1, is_hub_ustr, AssetRef, HubUsdConfig};
use super::pair_price_usd::{
    human_quote_per_base_from_reserves, mark_price_usd, quote_usd_kind, usd_per_human_quote,
    HubQuoteUsd, QuoteUsdKind,
};

#[derive(Debug, Clone, FromRow)]
struct MarkPairRow {
    pair_id: i32,
    a0_decimals: i16,
    a1_id: i32,
    a1_symbol: String,
    a1_denom: Option<String>,
    a1_contract: Option<String>,
    a1_is_cw20: bool,
    a1_decimals: i16,
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
    last_human: Option<BigDecimal>,
}

fn quote_asset_ref(row: &MarkPairRow) -> AssetRef {
    AssetRef {
        id: row.a1_id,
        symbol: row.a1_symbol.clone(),
        denom: row.a1_denom.clone(),
        contract_address: row.a1_contract.clone(),
        is_cw20: row.a1_is_cw20,
        decimals: row.a1_decimals,
    }
}

/// Catalog quote for idle marks (C568-6): hub contract/denom, not symbol spoofs.
fn mark_quote_kind(quote: &AssetRef, cfg: &HubUsdConfig) -> Option<QuoteUsdKind> {
    if is_hub_custc(quote, cfg) {
        return Some(QuoteUsdKind::Ustc);
    }
    if is_hub_ust1(quote, cfg) {
        return Some(QuoteUsdKind::Peg1);
    }
    if is_hub_ustr(quote, cfg) {
        return Some(QuoteUsdKind::Ustr);
    }
    if quote.denom.as_deref() == Some("uluna") {
        return Some(QuoteUsdKind::Lunc);
    }
    if !quote.is_cw20 {
        return None;
    }
    if quote
        .contract_address
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return None;
    }
    match quote_usd_kind(&quote.symbol, quote.denom.as_deref()) {
        Some(QuoteUsdKind::Ustc | QuoteUsdKind::Lunc) => {
            quote_usd_kind(&quote.symbol, quote.denom.as_deref())
        }
        Some(QuoteUsdKind::Peg1 | QuoteUsdKind::Ustr) | None => None,
    }
}

fn last_human_for_row(row: &MarkPairRow) -> Option<BigDecimal> {
    let zero = BigDecimal::from(0);
    if let Some(h) = row.last_human.as_ref().filter(|h| *h > &zero) {
        return Some(h.clone());
    }
    match (row.reserve_0.as_ref(), row.reserve_1.as_ref()) {
        (Some(r0), Some(r1)) => {
            human_quote_per_base_from_reserves(r0, r1, row.a0_decimals, row.a1_decimals)
        }
        _ => None,
    }
}

async fn list_active_pairs(pool: &PgPool) -> Result<Vec<MarkPairRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT
            p.id AS pair_id,
            a0.decimals AS a0_decimals,
            a1.id AS a1_id,
            a1.symbol AS a1_symbol,
            a1.denom AS a1_denom,
            a1.contract_address AS a1_contract,
            a1.is_cw20 AS a1_is_cw20,
            a1.decimals AS a1_decimals,
            r.reserve_0,
            r.reserve_1,
            ls.price AS last_human
         FROM pairs p
         JOIN assets a0 ON a0.id = p.asset_0_id
         JOIN assets a1 ON a1.id = p.asset_1_id
         LEFT JOIN pair_reserves r ON r.pair_id = p.id
         LEFT JOIN LATERAL (
            SELECT se.price
            FROM swap_events se
            WHERE se.pair_id = p.id AND se.price > 0
            ORDER BY se.block_timestamp DESC, se.id DESC
            LIMIT 1
         ) ls ON TRUE
         WHERE p.is_active",
    )
    .fetch_all(pool)
    .await
}

/// Materialize current-bucket USD marks for catalog-quoted factory pairs.
///
/// Bounded to active indexed pairs × candle intervals. Does not `UPDATE` historical
/// `swap_events` or past candle rows (only the truncated *now* bucket per interval).
pub async fn apply_idle_usd_marks(
    pool: &PgPool,
    cfg: &HubUsdConfig,
    now: DateTime<Utc>,
    ustc_usd: Option<&BigDecimal>,
    lunc_usd: Option<&BigDecimal>,
    hub: &HubQuoteUsd,
) -> Result<usize, sqlx::Error> {
    let pairs = list_active_pairs(pool).await?;
    let mut written = 0usize;
    for row in pairs {
        let quote = quote_asset_ref(&row);
        let Some(kind) = mark_quote_kind(&quote, cfg) else {
            continue;
        };
        let Some(quote_usd) = usd_per_human_quote(kind, ustc_usd, lunc_usd, Some(hub))
            .filter(|p| *p > BigDecimal::from(0))
        else {
            continue;
        };
        let Some(human) = last_human_for_row(&row) else {
            continue;
        };
        let Some(usd) = mark_price_usd(&human, &quote_usd) else {
            continue;
        };
        candle_builder::update_candles_for_mark(pool, row.pair_id, now, &usd, &human).await?;
        written += 1;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn cfg() -> HubUsdConfig {
        HubUsdConfig {
            custc_address: "terra1custc".into(),
            clunc_address: "terra1clunc".into(),
            ust1_address: "terra1ust1".into(),
            ustr_address: "terra1ustr".into(),
            cl8y_address: crate::config::DEFAULT_HUB_CL8Y_ADDRESS.to_string(),
            tvl_floor: BigDecimal::from(100),
            max_staleness: Duration::from_secs(60),
        }
    }

    fn cw20(symbol: &str, contract: &str) -> AssetRef {
        AssetRef {
            id: 1,
            symbol: symbol.into(),
            denom: None,
            contract_address: Some(contract.into()),
            is_cw20: true,
            decimals: 6,
        }
    }

    fn native(symbol: &str, denom: &str) -> AssetRef {
        AssetRef {
            id: 2,
            symbol: symbol.into(),
            denom: Some(denom.into()),
            contract_address: None,
            is_cw20: false,
            decimals: 6,
        }
    }

    #[test]
    fn mark_kind_uses_hub_contract_not_symbol() {
        let c = cfg();
        assert_eq!(
            mark_quote_kind(&cw20("cUSTC", "terra1custc"), &c),
            Some(QuoteUsdKind::Ustc)
        );
        assert_eq!(
            mark_quote_kind(&cw20("UST1", "terra1ust1"), &c),
            Some(QuoteUsdKind::Peg1)
        );
        assert_eq!(
            mark_quote_kind(&cw20("USTR", "terra1ustr"), &c),
            Some(QuoteUsdKind::Ustr)
        );
        assert!(mark_quote_kind(&cw20("UST1", "terra1fakeust1"), &c).is_none());
        assert!(mark_quote_kind(&cw20("USTR", "terra1clone"), &c).is_none());
        assert!(mark_quote_kind(&native("USTR", "ugem"), &c).is_none());
        assert_eq!(
            mark_quote_kind(&native("LUNC", "uluna"), &c),
            Some(QuoteUsdKind::Lunc)
        );
        assert_eq!(
            mark_quote_kind(&cw20("cLUNC", "terra1clunc"), &c),
            Some(QuoteUsdKind::Lunc)
        );
        assert!(mark_quote_kind(&cw20("GEMX", "terra1gem"), &c).is_none());
    }
}
