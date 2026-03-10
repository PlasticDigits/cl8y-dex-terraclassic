use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct PairRow {
    pub id: i32,
    pub contract_address: String,
    pub asset_0_id: i32,
    pub asset_1_id: i32,
    pub lp_token: Option<String>,
    pub fee_bps: Option<i16>,
    pub hooks: Vec<String>,
    pub created_at_block: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn upsert_pair(
    pool: &PgPool,
    contract_address: &str,
    asset_0_id: i32,
    asset_1_id: i32,
    lp_token: Option<&str>,
    fee_bps: Option<i16>,
    hooks: &[String],
    created_at_block: Option<i64>,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps, hooks, created_at_block)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (contract_address)
           DO UPDATE SET lp_token = COALESCE(EXCLUDED.lp_token, pairs.lp_token),
                        fee_bps = COALESCE(EXCLUDED.fee_bps, pairs.fee_bps),
                        hooks = EXCLUDED.hooks,
                        updated_at = NOW()
         RETURNING id",
    )
    .bind(contract_address)
    .bind(asset_0_id)
    .bind(asset_1_id)
    .bind(lp_token)
    .bind(fee_bps)
    .bind(hooks)
    .bind(created_at_block)
    .fetch_one(pool)
    .await
}

pub async fn get_pair_by_address(
    pool: &PgPool,
    contract_address: &str,
) -> Result<Option<PairRow>, sqlx::Error> {
    sqlx::query_as::<_, PairRow>("SELECT * FROM pairs WHERE contract_address = $1")
        .bind(contract_address)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_pairs(pool: &PgPool) -> Result<Vec<PairRow>, sqlx::Error> {
    sqlx::query_as::<_, PairRow>("SELECT * FROM pairs ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn get_pairs_for_asset(
    pool: &PgPool,
    asset_id: i32,
) -> Result<Vec<PairRow>, sqlx::Error> {
    sqlx::query_as::<_, PairRow>(
        "SELECT * FROM pairs WHERE asset_0_id = $1 OR asset_1_id = $1 ORDER BY id",
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
}

pub async fn update_pair_config(
    pool: &PgPool,
    pair_id: i32,
    fee_bps: Option<i16>,
    hooks: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE pairs SET fee_bps = COALESCE($2, fee_bps), hooks = $3, updated_at = NOW()
         WHERE id = $1",
    )
    .bind(pair_id)
    .bind(fee_bps)
    .bind(hooks)
    .execute(pool)
    .await?;
    Ok(())
}
