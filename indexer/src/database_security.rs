//! Separate schema management from least-privilege serving (#1396).
use sqlx::PgPool;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();
pub const QA_DATABASE_MARKER: &str = "cl8y-dex:isolated-qa:v1";
pub const PRE_COHORT_SCHEMA_VERSION: i64 = 20261016120000;
pub const COHORT_SCHEMA_VERSION: i64 = 20261018110000;

fn required_schema_version() -> i64 {
    MIGRATOR
        .iter()
        .filter(|m| !m.migration_type.is_down_migration())
        .map(|m| m.version)
        .max()
        .unwrap_or(0)
}

/// Only the dedicated migration process calls this; no server path carries migration credentials.
pub async fn migrate_and_grant(pool: &PgPool, runtime_role: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !runtime_role.is_empty()
            && runtime_role.len() <= 63
            && runtime_role
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_'),
        "INDEXER_RUNTIME_ROLE must be a SQL role identifier"
    );
    // Refuse a shared public schema; broad DML reconciliation must never reach another
    // application's objects or silently transfer existing ownership.
    let foreign_objects: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = 'public' AND c.relkind IN ('r','p','v','m','f','S')
         AND c.relowner <> (SELECT oid FROM pg_roles WHERE rolname = current_user))
         OR EXISTS (SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
         WHERE n.nspname = 'public' AND p.proowner <> (SELECT oid FROM pg_roles WHERE rolname = current_user))",
    )
    .fetch_one(pool)
    .await?;
    anyhow::ensure!(!foreign_objects, "public schema contains objects owned by another role; review database separation and ownership before migration");
    MIGRATOR.run(pool).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS public.indexer_schema_version (singleton BOOLEAN PRIMARY KEY CHECK (singleton), version BIGINT NOT NULL)").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO public.indexer_schema_version(singleton, version) VALUES (true, $1) ON CONFLICT(singleton) DO UPDATE SET version = EXCLUDED.version").bind(required_schema_version()).execute(&mut *tx).await?;
    let role = format!("\"{runtime_role}\"");
    // Reconcile new tables after every migration, without granting the SQLx ledger.
    for statement in [
        format!("REVOKE ALL ON SCHEMA public FROM PUBLIC, {role}"),
        format!("GRANT USAGE ON SCHEMA public TO {role}"),
        format!("REVOKE ALL ON ALL TABLES IN SCHEMA public FROM PUBLIC, {role}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {role}"),
        format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {role}"),
        format!("REVOKE ALL ON TABLE public._sqlx_migrations FROM {role}"),
        format!("REVOKE ALL ON TABLE public.indexer_schema_version FROM {role}"),
        format!("GRANT SELECT ON TABLE public.indexer_schema_version TO {role}"),
    ] {
        sqlx::query(&statement).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn verify_schema_ready(pool: &PgPool) -> anyhow::Result<()> {
    let version: i64 = sqlx::query_scalar("SELECT version FROM public.indexer_schema_version WHERE singleton = true")
        .fetch_one(pool).await.map_err(|_| anyhow::anyhow!("database schema is not prepared; run the standalone migrate command with migration credentials"))?;
    anyhow::ensure!(version == required_schema_version(), "database schema version differs from this binary; run the standalone migrate command before starting the server");
    Ok(())
}

fn serving_schema_supported(version: i64) -> bool {
    let required = required_schema_version();
    version == required
        || (required == COHORT_SCHEMA_VERSION && version == PRE_COHORT_SCHEMA_VERSION)
}

/// Serving alone supports the reviewed additive 67 -> 68 transition. A future
/// compiled migration restores strict startup until its compatibility is reviewed.
/// Call within the owner's snapshot; the result never grants migration authority.
pub async fn cohorts_ready<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
) -> Result<bool, sqlx::Error> {
    let version: i64 = sqlx::query_scalar(
        "SELECT version FROM public.indexer_schema_version WHERE singleton = true",
    )
    .fetch_one(executor)
    .await?;
    if !serving_schema_supported(version) {
        return Err(sqlx::Error::Protocol(
            "unsupported serving schema version".into(),
        ));
    }
    Ok(version >= COHORT_SCHEMA_VERSION)
}

async fn verify_unprivileged_role(pool: &PgPool) -> anyhow::Result<()> {
    let safe: bool = sqlx::query_scalar(
        "SELECT current_user = session_user AND NOT (r.rolsuper OR r.rolcreatedb OR r.rolcreaterole OR r.rolreplication OR r.rolbypassrls)
         AND r.oid <> d.datdba
         AND NOT EXISTS (SELECT 1 FROM pg_roles parent WHERE parent.oid <> r.oid AND pg_has_role(r.oid, parent.oid, 'MEMBER'))
         FROM pg_roles r JOIN pg_database d ON d.datname = current_database() WHERE r.rolname = current_user"
    ).fetch_one(pool).await?;
    anyhow::ensure!(safe, "database role has administrative privileges, database ownership or inherited roles; use a dedicated least-privilege credential");
    Ok(())
}

async fn verify_runtime_permissions(pool: &PgPool) -> anyhow::Result<()> {
    verify_unprivileged_role(pool).await?;
    let database_create: bool = sqlx::query_scalar(
        "SELECT has_database_privilege(current_user, current_database(), 'CREATE')",
    )
    .fetch_one(pool)
    .await?;
    anyhow::ensure!(
        !database_create,
        "runtime database role may create schemas/extensions; revoke database CREATE"
    );
    let unrelated_connect: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_database d WHERE d.datname <> current_database()
         AND d.datallowconn AND NOT d.datistemplate AND has_database_privilege(current_user, d.oid, 'CONNECT'))"
    ).fetch_one(pool).await?;
    anyhow::ensure!(!unrelated_connect, "runtime database role can connect to an unrelated database; review PUBLIC and role connection grants");
    let unsafe_objects: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_namespace n
         WHERE n.nspname <> 'information_schema' AND left(n.nspname, 3) <> 'pg_'
         AND (n.nspowner = (SELECT oid FROM pg_roles WHERE rolname = current_user)
              OR has_schema_privilege(current_user, n.oid, 'CREATE')
              OR (n.nspname <> 'public' AND has_schema_privilege(current_user, n.oid, 'USAGE'))))
         OR EXISTS (SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname <> 'information_schema' AND left(n.nspname, 3) <> 'pg_'
         AND (c.relowner = (SELECT oid FROM pg_roles WHERE rolname = current_user)
              OR CASE WHEN n.nspname <> 'public' AND c.relkind IN ('r','p','v','m','f')
                  THEN has_table_privilege(current_user, c.oid, 'INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER') ELSE false END))
         OR EXISTS (SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
         WHERE n.nspname <> 'information_schema' AND left(n.nspname, 3) <> 'pg_'
         AND (p.proowner = (SELECT oid FROM pg_roles WHERE rolname = current_user)
              OR (p.prosecdef AND has_function_privilege(current_user, p.oid, 'EXECUTE'))))"
    ).fetch_one(pool).await?;
    anyhow::ensure!(!unsafe_objects, "runtime database role owns schema objects, can create objects, can access another schema or can execute privileged functions; reconcile least-privilege grants");
    let ledger_access: bool = sqlx::query_scalar("SELECT has_table_privilege(current_user, 'public._sqlx_migrations', 'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')").fetch_one(pool).await?;
    anyhow::ensure!(
        !ledger_access,
        "runtime database role can access migration history; run migrate grant reconciliation"
    );
    Ok(())
}

/// Strict entry point for QA, maintenance and callers outside server startup.
pub async fn verify_runtime(pool: &PgPool) -> anyhow::Result<()> {
    verify_runtime_permissions(pool).await?;
    verify_schema_ready(pool).await
}

/// Identical least-privilege checks, with only the reviewed serving version pair.
pub async fn verify_serving_runtime(pool: &PgPool) -> anyhow::Result<()> {
    verify_runtime_permissions(pool).await?;
    cohorts_ready(pool).await?;
    Ok(())
}

pub async fn verify_qa(pool: &PgPool, expected_database: &str) -> anyhow::Result<()> {
    let safe: bool = sqlx::query_scalar(
        "SELECT current_user = session_user AND rolname = 'dex_indexer_qa' AND NOT (rolsuper OR rolcreaterole OR rolreplication OR rolbypassrls)
         AND NOT EXISTS (SELECT 1 FROM pg_auth_members WHERE member = r.oid)
         FROM pg_roles r WHERE rolname = current_user"
    ).fetch_one(pool).await?;
    anyhow::ensure!(safe, "seed-qa requires the dedicated QA role without administrative privileges or inherited roles");
    let (name, marker): (String, Option<String>) = sqlx::query_as("SELECT datname, shobj_description(oid, 'pg_database') FROM pg_database WHERE datname = current_database()").fetch_one(pool).await?;
    anyhow::ensure!(
        name == expected_database && marker.as_deref() == Some(QA_DATABASE_MARKER),
        "database lacks the administrator-created isolated QA identity marker; seed/clean refused"
    );
    verify_schema_ready(pool).await
}
