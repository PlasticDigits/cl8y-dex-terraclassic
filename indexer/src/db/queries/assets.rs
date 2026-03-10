use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct AssetRow {
    pub id: i32,
    pub contract_address: Option<String>,
    pub denom: Option<String>,
    pub is_cw20: bool,
    pub name: String,
    pub symbol: String,
    pub decimals: i16,
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
    pub cmc_id: Option<i32>,
    pub first_seen_block: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn upsert_asset(
    pool: &PgPool,
    contract_address: Option<&str>,
    denom: Option<&str>,
    is_cw20: bool,
    name: &str,
    symbol: &str,
    decimals: i16,
    first_seen_block: Option<i64>,
) -> Result<i32, sqlx::Error> {
    let row = sqlx::query_scalar::<_, i32>(
        "INSERT INTO assets (contract_address, denom, is_cw20, name, symbol, decimals, first_seen_block)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (contract_address) WHERE contract_address IS NOT NULL
           DO UPDATE SET name = EXCLUDED.name, symbol = EXCLUDED.symbol,
                        decimals = EXCLUDED.decimals, updated_at = NOW()
         RETURNING id",
    )
    .bind(contract_address)
    .bind(denom)
    .bind(is_cw20)
    .bind(name)
    .bind(symbol)
    .bind(decimals)
    .bind(first_seen_block)
    .fetch_one(pool)
    .await;

    match row {
        Ok(id) => Ok(id),
        Err(_) => {
            // Retry with denom conflict for native tokens
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO assets (contract_address, denom, is_cw20, name, symbol, decimals, first_seen_block)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (denom) WHERE denom IS NOT NULL
                   DO UPDATE SET name = EXCLUDED.name, symbol = EXCLUDED.symbol,
                                decimals = EXCLUDED.decimals, updated_at = NOW()
                 RETURNING id",
            )
            .bind(contract_address)
            .bind(denom)
            .bind(is_cw20)
            .bind(name)
            .bind(symbol)
            .bind(decimals)
            .bind(first_seen_block)
            .fetch_one(pool)
            .await
        }
    }
}

pub async fn get_asset_by_contract(
    pool: &PgPool,
    contract_address: &str,
) -> Result<Option<AssetRow>, sqlx::Error> {
    sqlx::query_as::<_, AssetRow>("SELECT * FROM assets WHERE contract_address = $1")
        .bind(contract_address)
        .fetch_optional(pool)
        .await
}

pub async fn get_asset_by_denom(
    pool: &PgPool,
    denom: &str,
) -> Result<Option<AssetRow>, sqlx::Error> {
    sqlx::query_as::<_, AssetRow>("SELECT * FROM assets WHERE denom = $1")
        .bind(denom)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_assets(pool: &PgPool) -> Result<Vec<AssetRow>, sqlx::Error> {
    sqlx::query_as::<_, AssetRow>("SELECT * FROM assets ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn get_asset_by_id(pool: &PgPool, id: i32) -> Result<Option<AssetRow>, sqlx::Error> {
    sqlx::query_as::<_, AssetRow>("SELECT * FROM assets WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn update_asset_metadata(
    pool: &PgPool,
    id: i32,
    logo_url: Option<&str>,
    coingecko_id: Option<&str>,
    cmc_id: Option<i32>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE assets SET logo_url = COALESCE($2, logo_url),
                          coingecko_id = COALESCE($3, coingecko_id),
                          cmc_id = COALESCE($4, cmc_id),
                          updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .bind(logo_url)
    .bind(coingecko_id)
    .bind(cmc_id)
    .execute(pool)
    .await?;
    Ok(())
}
