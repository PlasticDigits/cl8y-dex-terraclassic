use sqlx::PgPool;

pub async fn get_state(pool: &PgPool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT value FROM indexer_state WHERE key = $1",
    )
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
    let val = get_state(pool, "last_indexed_height").await?;
    Ok(val.and_then(|v| v.parse().ok()).unwrap_or(0))
}

pub async fn set_last_indexed_height(pool: &PgPool, height: i64) -> Result<(), sqlx::Error> {
    set_state(pool, "last_indexed_height", &height.to_string()).await
}
