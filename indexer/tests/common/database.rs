//! #1396: each integration binary owns a fresh database and migration history.
//!
//! Never migrate the supplied control database, accept runtime credentials, or
//! ignore missing migrations. Tests within a binary still use `#[serial]` or
//! `--test-threads=1`; independent binaries/worktrees do not share mutable state.

use std::{env, str::FromStr, sync::OnceLock};

use sqlx::{postgres::PgConnectOptions, PgPool};
use tokio::sync::OnceCell;

static DATABASE: OnceCell<PgConnectOptions> = OnceCell::const_new();
static DATABASE_URL: OnceLock<String> = OnceLock::new();

pub fn test_database_url() -> String {
    DATABASE_URL.get().cloned().unwrap_or_else(|| {
        env::var("TEST_DATABASE_ADMIN_URL")
            .or_else(|_| env::var("TEST_DATABASE_URL"))
            .unwrap_or_default()
    })
}

fn admin_options(url: &str) -> Result<PgConnectOptions, &'static str> {
    // Reuse the production seed preflight: parse without leaking query values,
    // require local host/dedicated QA login, and match this exact control DB.
    cl8y_dex_indexer::qa_guard::validate_seed_target(Some("qa"), url, Some("dex_indexer_test"))
        .map_err(|_| "integration tests require the dedicated local marked QA control database URL without session overrides")?;
    PgConnectOptions::from_str(url).map_err(|_| "invalid QA database URL")
}

pub async fn setup_pool() -> PgPool {
    let options = DATABASE
        .get_or_init(|| async {
            let url = env::var("TEST_DATABASE_ADMIN_URL")
                .or_else(|_| env::var("TEST_DATABASE_URL"))
                .expect("set TEST_DATABASE_ADMIN_URL in indexer/.env.qa; run make setup-indexer-postgres");
            let admin = admin_options(&url).unwrap_or_else(|error| panic!("{error}"));
            let control = PgPool::connect_with(admin.clone())
                .await
                .expect("connect QA control database (URL omitted to protect credentials)");
            let dedicated: bool = sqlx::query_scalar(
                "SELECT session_user = current_user AND current_user = 'dex_indexer_qa' AND rolcreatedb AND NOT rolsuper
                 AND NOT rolcreaterole AND NOT rolreplication AND NOT rolbypassrls AND NOT rolinherit
                 AND NOT EXISTS (SELECT 1 FROM pg_auth_members WHERE member = r.oid)
                 FROM pg_roles r WHERE rolname = current_user",
            )
            .fetch_one(&control)
            .await
            .expect("verify dedicated QA role");
            assert!(dedicated, "QA role must have only LOGIN/CREATEDB and no role memberships");
            let marked: bool = sqlx::query_scalar(
                "SELECT COALESCE(shobj_description(oid, 'pg_database') = 'cl8y-dex:isolated-qa:v1', false)
                 AND pg_get_userbyid(datdba) <> current_user
                 FROM pg_database WHERE datname = current_database()",
            )
            .fetch_one(&control)
            .await
            .expect("verify admin-provisioned QA database marker");
            assert!(marked, "QA control database requires an admin-owned isolation marker");
            let name = format!(
                "dex_indexer_test_{}_{:016x}",
                std::process::id(),
                rand::random::<u64>()
            );
            // Identifier consists solely of a constant ASCII prefix, PID and hex.
            sqlx::query(&format!("CREATE DATABASE {name} TEMPLATE template0 ALLOW_CONNECTIONS false"))
                .execute(&control)
                .await
                .expect("create disposable integration database");
            sqlx::query(&format!("REVOKE CONNECT ON DATABASE {name} FROM PUBLIC"))
                .execute(&control)
                .await
                .expect("restrict disposable integration database");
            sqlx::query(&format!(
                "COMMENT ON DATABASE {name} IS 'cl8y-dex:isolated-qa:v1'"
            ))
            .execute(&control)
            .await
            .expect("mark disposable database for explicit cleanup");
            sqlx::query(&format!("ALTER DATABASE {name} ALLOW_CONNECTIONS true"))
                .execute(&control)
                .await
                .expect("enable connections only after disposable database ACL setup");
            control.close().await;
            let options = admin.database(&name);
            // PgConnectOptions::to_url_lossy adds default query parameters,
            // which seed/runtime preflight deliberately rejects. Preserve the
            // validated original authority and replace only its database path.
            let mut isolated_url = reqwest::Url::parse(&url).expect("validated QA URL");
            isolated_url.set_path(&name);
            DATABASE_URL
                .set(isolated_url.to_string())
                .expect("initialize process database URL once");
            options
        })
        .await;
    let pool = PgPool::connect_with(options.clone())
        .await
        .expect("connect disposable integration database");
    assert_owned_pool(&pool).await;
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("strict migrations on fresh disposable integration database");
    pool
}

pub async fn assert_owned_pool(pool: &PgPool) {
    let expected = DATABASE
        .get()
        .and_then(PgConnectOptions::get_database)
        .expect("test helper cannot mutate a pool before isolated setup");
    let (database, owner): (String, String) = sqlx::query_as(
        "SELECT current_database(), pg_get_userbyid(datdba)
         FROM pg_database WHERE datname = current_database()",
    )
    .fetch_one(pool)
    .await
    .expect("verify disposable test database identity");
    assert_eq!(
        database, expected,
        "refusing mutation of an unrelated database"
    );
    assert_eq!(owner, "dex_indexer_qa", "test database must be owned by QA");
}

#[cfg(test)]
mod tests {
    use super::admin_options;

    #[test]
    fn production_runtime_and_superuser_urls_are_rejected_without_connecting() {
        for url in [
            "postgres://dex_indexer_qa:secret@127.0.0.1:1/dex_indexer",
            "postgres://dex_indexer_runtime:secret@127.0.0.1:1/postgres",
            "postgres://dex_indexer_migrator:secret@127.0.0.1:1/postgres",
            "postgres://postgres:secret@127.0.0.1:1/dex_indexer_test",
            "postgres://dex_indexer_qa:secret@127.0.0.1:1/another_app",
            "postgres://dex_indexer_qa:secret@remote.example/dex_indexer_test",
            "postgres://dex_indexer_qa:secret@localhost/dex_indexer_test?options=-c%20role%3Ddex_indexer_qa",
        ] {
            assert!(admin_options(url).is_err());
        }
        assert!(
            admin_options("postgres://dex_indexer_qa:secret@127.0.0.1:1/dex_indexer_test").is_ok()
        );
    }
}
