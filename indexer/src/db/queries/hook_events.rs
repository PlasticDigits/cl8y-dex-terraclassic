use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn hook_event_exists(
    pool: &PgPool,
    tx_hash: &str,
    hook_address: &str,
    action: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM hook_events WHERE tx_hash = $1 AND hook_address = $2 AND action = $3)"
    )
    .bind(tx_hash)
    .bind(hook_address)
    .bind(action)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn insert_hook_event(
    pool: &PgPool,
    tx_hash: &str,
    hook_address: &str,
    action: &str,
    amount: Option<&BigDecimal>,
    token: Option<&str>,
    skipped: Option<&str>,
    warning: Option<&str>,
    block_height: i64,
    block_time: DateTime<Utc>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query(
        r#"INSERT INTO hook_events (tx_hash, hook_address, action, amount, token, skipped, warning, block_height, block_time)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(tx_hash)
    .bind(hook_address)
    .bind(action)
    .bind(amount)
    .bind(token)
    .bind(skipped)
    .bind(warning)
    .bind(block_height)
    .bind(block_time)
    .execute(pool)
    .await?;
    Ok(())
}
