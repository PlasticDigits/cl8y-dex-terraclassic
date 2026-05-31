use sqlx::PgPool;

pub const KEY_LAST_INDEXED_HEIGHT: &str = "last_indexed_height";
pub const KEY_LAST_INDEXED_BLOCK_HASH: &str = "last_indexed_block_hash";

pub async fn get_state(pool: &PgPool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_scalar::<_, String>("SELECT value FROM indexer_state WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn set_state(pool: &PgPool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO indexer_state (key, value, updated_at) VALUES ($1, $2, NOW())
         ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_last_indexed_height(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let val = get_state(pool, KEY_LAST_INDEXED_HEIGHT).await?;
    Ok(val.and_then(|v| v.parse().ok()).unwrap_or(0))
}

pub async fn get_last_indexed_block_hash(pool: &PgPool) -> Result<Option<String>, sqlx::Error> {
    get_state(pool, KEY_LAST_INDEXED_BLOCK_HASH).await
}

pub async fn set_last_indexed_height(pool: &PgPool, height: i64) -> Result<(), sqlx::Error> {
    set_state(pool, KEY_LAST_INDEXED_HEIGHT, &height.to_string()).await
}

pub async fn set_indexer_checkpoint(
    pool: &PgPool,
    height: i64,
    block_hash: &str,
) -> Result<(), sqlx::Error> {
    set_last_indexed_height(pool, height).await?;
    set_state(pool, KEY_LAST_INDEXED_BLOCK_HASH, block_hash).await
}

pub async fn record_failed_block(
    pool: &PgPool,
    height: i64,
    error_message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO indexer_failed_blocks (height, error_message, retry_count, first_failed_at, last_failed_at)
         VALUES ($1, $2, 1, NOW(), NOW())
         ON CONFLICT (height) DO UPDATE SET
           error_message = EXCLUDED.error_message,
           retry_count = indexer_failed_blocks.retry_count + 1,
           last_failed_at = NOW()",
    )
    .bind(height)
    .bind(error_message)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear_failed_block(pool: &PgPool, height: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM indexer_failed_blocks WHERE height = $1")
        .bind(height)
        .execute(pool)
        .await?;
    Ok(())
}
