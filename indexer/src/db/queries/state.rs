use sqlx::PgPool;

pub const KEY_LAST_INDEXED_HEIGHT: &str = "last_indexed_height";
pub const KEY_LAST_INDEXED_BLOCK_HASH: &str = "last_indexed_block_hash";

/// Session advisory-lock key so only one poller writes checkpoints per database.
/// Held on a dedicated `PgConnection` (not a pool checkout) until that session closes.
pub const POLLER_ADVISORY_LOCK_KEY: i64 = 0x434C_3859_4944_5801;

/// Try to take the poller lock on this session. Exclusive: a second indexer must not ingest.
pub async fn try_acquire_poller_lock(conn: &mut sqlx::PgConnection) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
        .bind(POLLER_ADVISORY_LOCK_KEY)
        .fetch_one(&mut *conn)
        .await
}

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

/// Read height + hash in one statement so the reorg guard cannot compare hash(H+1) to LCD(H)
/// when another poller advanced the cursor (Coolify rolling rebuild overlap).
pub async fn get_indexer_checkpoint(pool: &PgPool) -> Result<(i64, Option<String>), sqlx::Error> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM indexer_state WHERE key IN ($1, $2)")
            .bind(KEY_LAST_INDEXED_HEIGHT)
            .bind(KEY_LAST_INDEXED_BLOCK_HASH)
            .fetch_all(pool)
            .await?;

    let mut height = 0i64;
    let mut hash = None;
    for (key, value) in rows {
        if key == KEY_LAST_INDEXED_HEIGHT {
            height = value.parse().unwrap_or(0);
        } else if key == KEY_LAST_INDEXED_BLOCK_HASH {
            hash = Some(value).filter(|h| !h.is_empty());
        }
    }
    Ok((height, hash))
}

pub async fn set_last_indexed_height(pool: &PgPool, height: i64) -> Result<(), sqlx::Error> {
    set_state(pool, KEY_LAST_INDEXED_HEIGHT, &height.to_string()).await
}

/// Atomically update height + hash so a crash or concurrent writer cannot leave
/// `last_indexed_height` and `last_indexed_block_hash` pointing at different blocks.
pub async fn set_indexer_checkpoint(
    pool: &PgPool,
    height: i64,
    block_hash: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let height_str = height.to_string();
    sqlx::query(
        "INSERT INTO indexer_state (key, value, updated_at) VALUES ($1, $2, NOW())
         ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
    )
    .bind(KEY_LAST_INDEXED_HEIGHT)
    .bind(&height_str)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO indexer_state (key, value, updated_at) VALUES ($1, $2, NOW())
         ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
    )
    .bind(KEY_LAST_INDEXED_BLOCK_HASH)
    .bind(block_hash)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
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
