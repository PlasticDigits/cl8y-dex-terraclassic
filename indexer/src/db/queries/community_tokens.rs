use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct CommunityTokenRow {
    pub contract_address: String,
    pub code_id: Option<i64>,
    pub wasm_admin: Option<String>,
    pub manager: Option<String>,
    pub launcher_address: Option<String>,
    pub buy_bps: Option<i16>,
    pub sell_bps: Option<i16>,
    pub transfer_bps: Option<i16>,
    pub features: Value,
    pub treasury: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<i16>,
    pub attested_cmm: bool,
    pub launcher_tx: Option<String>,
    pub instantiate_tx: Option<String>,
    pub created_at_block: Option<i64>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CommunityTokenEventRow {
    pub id: i64,
    pub contract_address: String,
    pub txhash: String,
    pub block_height: i64,
    pub action: String,
    pub kind: String,
    pub sku: Option<String>,
    pub invoice: Option<String>,
    pub attrs: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityTokenFeatures {
    #[serde(default)]
    pub mint_control: bool,
    #[serde(default)]
    pub transfer_tax: bool,
    #[serde(default)]
    pub split_router: bool,
    #[serde(default)]
    pub auto_v2_lp: bool,
    #[serde(default)]
    pub exemption_directory: bool,
    #[serde(default)]
    pub variable_rates: bool,
    #[serde(default)]
    pub launch_guards: bool,
}

pub async fn upsert_from_launcher(
    pool: &PgPool,
    contract_address: &str,
    launcher_address: &str,
    launcher_tx: &str,
    height: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO community_tokens (
            contract_address, launcher_address, launcher_tx, instantiate_tx, created_at_block, updated_at
        )
        VALUES ($1, $2, $3, $3, $4, NOW())
        ON CONFLICT (contract_address) DO UPDATE SET
            launcher_address = EXCLUDED.launcher_address,
            launcher_tx = COALESCE(community_tokens.launcher_tx, EXCLUDED.launcher_tx),
            instantiate_tx = COALESCE(community_tokens.instantiate_tx, EXCLUDED.instantiate_tx),
            created_at_block = COALESCE(community_tokens.created_at_block, EXCLUDED.created_at_block),
            updated_at = NOW()
        "#,
    )
    .bind(contract_address)
    .bind(launcher_address)
    .bind(launcher_tx)
    .bind(height)
    .execute(pool)
    .await?;
    Ok(())
}

/// Adopt ingest (#626): insert the row without `launcher_tx`.
pub async fn upsert_from_migrate(
    pool: &PgPool,
    contract_address: &str,
    instantiate_tx: &str,
    height: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO community_tokens (
            contract_address, instantiate_tx, created_at_block, updated_at
        )
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (contract_address) DO UPDATE SET
            instantiate_tx = COALESCE(community_tokens.instantiate_tx, EXCLUDED.instantiate_tx),
            created_at_block = COALESCE(community_tokens.created_at_block, EXCLUDED.created_at_block),
            updated_at = NOW()
        "#,
    )
    .bind(contract_address)
    .bind(instantiate_tx)
    .bind(height)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn apply_lcd_snapshot(
    pool: &PgPool,
    contract_address: &str,
    code_id: i64,
    wasm_admin: Option<&str>,
    manager: Option<&str>,
    treasury: Option<&str>,
    buy_bps: Option<i16>,
    sell_bps: Option<i16>,
    transfer_bps: Option<i16>,
    features: &Value,
    name: Option<&str>,
    symbol: Option<&str>,
    decimals: Option<i16>,
    attested_cmm: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE community_tokens SET
            code_id = $2,
            wasm_admin = $3,
            manager = $4,
            treasury = $5,
            buy_bps = $6,
            sell_bps = $7,
            transfer_bps = $8,
            features = $9,
            name = COALESCE($10, name),
            symbol = COALESCE($11, symbol),
            decimals = COALESCE($12, decimals),
            attested_cmm = $13,
            updated_at = NOW()
        WHERE contract_address = $1
        "#,
    )
    .bind(contract_address)
    .bind(code_id)
    .bind(wasm_admin)
    .bind(manager)
    .bind(treasury)
    .bind(buy_bps)
    .bind(sell_bps)
    .bind(transfer_bps)
    .bind(features)
    .bind(name)
    .bind(symbol)
    .bind(decimals)
    .bind(attested_cmm)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn merge_feature(
    pool: &PgPool,
    contract_address: &str,
    sku: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE community_tokens
        SET features = COALESCE(features, '{}'::jsonb) || jsonb_build_object($2::text, true),
            updated_at = NOW()
        WHERE contract_address = $1
        "#,
    )
    .bind(contract_address)
    .bind(sku)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_event(
    pool: &PgPool,
    contract_address: &str,
    txhash: &str,
    block_height: i64,
    action: &str,
    kind: &str,
    sku: Option<&str>,
    invoice: Option<&str>,
    attrs: Option<&Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO community_token_events (
            contract_address, txhash, block_height, action, kind, sku, invoice, attrs
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (txhash, action, contract_address, kind) DO NOTHING
        "#,
    )
    .bind(contract_address)
    .bind(txhash)
    .bind(block_height)
    .bind(action)
    .bind(kind)
    .bind(sku)
    .bind(invoice)
    .bind(attrs)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_tokens(
    pool: &PgPool,
    manager: Option<&str>,
    include_unattested: bool,
    limit: i64,
    offset: i64,
) -> Result<(Vec<CommunityTokenRow>, i64), sqlx::Error> {
    let manager_l = manager.map(|m| m.to_ascii_lowercase());
    let rows = sqlx::query_as::<_, CommunityTokenRow>(
        r#"
        SELECT * FROM community_tokens
        WHERE ($1::text IS NULL OR lower(manager) = $1)
          AND ($2 OR attested_cmm)
        ORDER BY created_at_block DESC NULLS LAST, contract_address
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(manager_l.as_deref())
    .bind(include_unattested)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint FROM community_tokens
        WHERE ($1::text IS NULL OR lower(manager) = $1)
          AND ($2 OR attested_cmm)
        "#,
    )
    .bind(manager_l.as_deref())
    .bind(include_unattested)
    .fetch_one(pool)
    .await?;

    Ok((rows, total.0))
}

pub async fn get_by_address(
    pool: &PgPool,
    addr: &str,
) -> Result<Option<CommunityTokenRow>, sqlx::Error> {
    sqlx::query_as::<_, CommunityTokenRow>(
        "SELECT * FROM community_tokens WHERE lower(contract_address) = lower($1)",
    )
    .bind(addr)
    .fetch_optional(pool)
    .await
}

/// Batch catalog lookup for route/solve tax ranking (GitLab #615). Empty `addrs` → empty vec.
pub async fn get_by_addresses(
    pool: &PgPool,
    addrs: &[String],
) -> Result<Vec<CommunityTokenRow>, sqlx::Error> {
    if addrs.is_empty() {
        return Ok(vec![]);
    }
    let lowered: Vec<String> = addrs
        .iter()
        .map(|a| a.trim().to_ascii_lowercase())
        .collect();
    sqlx::query_as::<_, CommunityTokenRow>(
        "SELECT * FROM community_tokens WHERE lower(contract_address) = ANY($1)",
    )
    .bind(&lowered)
    .fetch_all(pool)
    .await
}

pub async fn list_events(
    pool: &PgPool,
    addr: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<CommunityTokenEventRow>, sqlx::Error> {
    sqlx::query_as::<_, CommunityTokenEventRow>(
        r#"
        SELECT * FROM community_token_events
        WHERE lower(contract_address) = lower($1)
        ORDER BY id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(addr)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn list_all_addresses(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT contract_address FROM community_tokens ORDER BY contract_address")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
