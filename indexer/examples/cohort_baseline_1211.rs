//! Owned-local rehearsal fixture: real SQLx migrations through the prior schema.
//! Never edits SQLx history. Refuses a non-loopback or unmarked/nonempty DB.
use cl8y_dex_indexer::database_security::MIGRATOR;
use sqlx::postgres::PgPoolOptions;
use std::borrow::Cow;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")?;
    let parsed = reqwest::Url::parse(&url)?;
    anyhow::ensure!(
        matches!(parsed.host_str(), Some("127.0.0.1" | "localhost")),
        "owned loopback fixture required"
    );
    let marker = std::env::var("COHORT_REHEARSAL_MARKER")?;
    anyhow::ensure!(
        marker.starts_with("cl8y-dex:cohort-rehearsal:") && marker.len() > 40,
        "owned fixture marker required"
    );
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let actual: Option<String> = sqlx::query_scalar("SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()")
        .fetch_one(&pool).await?;
    anyhow::ensure!(
        actual.as_deref() == Some(marker.as_str()),
        "wrong fixture marker"
    );
    let (effective, session): (String, String) =
        sqlx::query_as("SELECT current_user::text,session_user::text")
            .fetch_one(&pool)
            .await?;
    if std::env::var("COHORT_IDENTITY_ONLY").as_deref() == Ok("1") {
        println!(
            "{}",
            serde_json::json!({"effective_migrator":effective=="dex_indexer_migrator_20261006","bootstrap_session":session=="postgres"})
        );
        return Ok(());
    }
    anyhow::ensure!(
        effective == "dex_indexer_migrator_20261006" && session == "postgres",
        "DSN options must establish effective owner"
    );
    let empty: bool = sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NULL")
        .fetch_one(&pool)
        .await?;
    anyhow::ensure!(empty, "baseline only accepts an empty owned database");
    let migrations = MIGRATOR
        .iter()
        .filter(|m| m.version < 20261018110000 && !m.migration_type.is_down_migration())
        .cloned()
        .collect::<Vec<_>>();
    let version = migrations
        .iter()
        .map(|m| m.version)
        .max()
        .ok_or_else(|| anyhow::anyhow!("missing baseline migrations"))?;
    anyhow::ensure!(
        migrations.len() == 67 && version == 20261016120000,
        "baseline prefix changed"
    );
    sqlx::migrate::Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
    .run(&pool)
    .await?;
    // Initialize the fresh fixture's readiness/grants exactly as the standalone
    // migration process does, deriving its version from the executed prefix.
    let mut tx = pool.begin().await?;
    sqlx::query("CREATE TABLE public.indexer_schema_version(singleton BOOLEAN PRIMARY KEY CHECK(singleton),version BIGINT NOT NULL)")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO public.indexer_schema_version VALUES(true,$1)")
        .bind(version)
        .execute(&mut *tx)
        .await?;
    for sql in [
        "REVOKE ALL ON SCHEMA public FROM PUBLIC,dex_indexer_runtime_20261006",
        "GRANT USAGE ON SCHEMA public TO dex_indexer_runtime_20261006",
        "REVOKE ALL ON ALL TABLES IN SCHEMA public FROM PUBLIC,dex_indexer_runtime_20261006",
        "GRANT SELECT,INSERT,UPDATE,DELETE ON ALL TABLES IN SCHEMA public TO dex_indexer_runtime_20261006",
        "GRANT USAGE,SELECT ON ALL SEQUENCES IN SCHEMA public TO dex_indexer_runtime_20261006",
        "REVOKE ALL ON TABLE public._sqlx_migrations,public.indexer_schema_version FROM dex_indexer_runtime_20261006",
        "GRANT SELECT ON TABLE public.indexer_schema_version TO dex_indexer_runtime_20261006",
    ] { sqlx::query(sql).execute(&mut *tx).await?; }
    tx.commit().await?;
    println!(
        "{}",
        serde_json::json!({"baseline_migrations":67,"schema_marker":version,"sqlx_history_manually_edited":false,"effective_migrator_verified":true})
    );
    Ok(())
}
