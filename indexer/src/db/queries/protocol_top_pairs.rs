//! Protocol top-5 pair ranking from `pair_volume_30d` + `pair_liquidity_usd` (Forgejo #1263).
//!
//! GET JOINs these stamps only — never `swap_events` / `pair_reserves`.

use bigdecimal::BigDecimal;
use sqlx::{FromRow, PgPool};

/// Rollup-only ranking SQL. `$1` is lowercased gem CW20 addresses (`COLUMBUS5_GEM_ADDRESSES`).
pub const LIST_TOP_PAIRS_SQL: &str = r#"
SELECT
    p.id AS pair_id,
    p.contract_address AS pair_address,
    a0.symbol AS asset_0_symbol,
    a0.contract_address AS asset_0_contract,
    a0.denom AS asset_0_denom,
    a0.decimals AS asset_0_decimals,
    a1.symbol AS asset_1_symbol,
    a1.contract_address AS asset_1_contract,
    a1.denom AS asset_1_denom,
    a1.decimals AS asset_1_decimals,
    pv.volume_usd AS volume_usd_30d,
    pl.liquidity_usd
FROM pair_volume_30d pv
INNER JOIN pairs p ON p.id = pv.pair_id
INNER JOIN assets a0 ON a0.id = p.asset_0_id
INNER JOIN assets a1 ON a1.id = p.asset_1_id
LEFT JOIN pair_liquidity_usd pl ON pl.pair_id = p.id
WHERE pv.volume_usd IS NOT NULL
  AND pv.volume_usd > 0
  AND NOT (
        LOWER(COALESCE(a0.contract_address, '')) = ANY($1)
     OR LOWER(COALESCE(a1.contract_address, '')) = ANY($1)
  )
ORDER BY pv.volume_usd DESC NULLS LAST, p.id ASC
LIMIT 5
"#;

#[derive(Debug, Clone, FromRow)]
pub struct ProtocolTopPairRow {
    pub pair_id: i32,
    pub pair_address: String,
    pub asset_0_symbol: String,
    pub asset_0_contract: Option<String>,
    pub asset_0_denom: Option<String>,
    pub asset_0_decimals: i16,
    pub asset_1_symbol: String,
    pub asset_1_contract: Option<String>,
    pub asset_1_denom: Option<String>,
    pub asset_1_decimals: i16,
    pub volume_usd_30d: BigDecimal,
    pub liquidity_usd: Option<BigDecimal>,
}

pub async fn list_top_pairs_30d(
    pool: &PgPool,
    gem_addresses_lowercased: &[String],
) -> Result<Vec<ProtocolTopPairRow>, sqlx::Error> {
    sqlx::query_as(LIST_TOP_PAIRS_SQL)
        .bind(gem_addresses_lowercased)
        .fetch_all(pool)
        .await
}
