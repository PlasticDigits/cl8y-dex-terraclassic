//! #1396: parallel binaries never migrate or clean each other's history.

mod common;

use sqlx::PgPool;

#[tokio::test]
#[serial_test::serial]
async fn parallel_processes_own_distinct_databases_and_preserve_control_history() {
    let pool = common::setup_pool().await;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap();
    if std::env::var_os("ISSUE_1396_ISOLATION_CHILD").is_some() {
        println!("ISSUE_1396_DATABASE={database}");
        return;
    }

    let control_url = std::env::var("TEST_DATABASE_ADMIN_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap();
    let control = PgPool::connect(&control_url).await.unwrap();
    async fn history_snapshot(pool: &PgPool) -> Option<Vec<(i64, String, bool, Vec<u8>, i64)>> {
        let history: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations')::text")
                .fetch_one(pool)
                .await
                .unwrap();
        match history {
            Some(_) => Some(sqlx::query_as(
                "SELECT version, description, success, checksum, execution_time FROM public._sqlx_migrations ORDER BY version",
            ).fetch_all(pool).await.unwrap()),
            None => None,
        }
    }
    let control_history_before = history_snapshot(&control).await;
    sqlx::query("CREATE TABLE isolation_sentinel (value integer)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO isolation_sentinel VALUES (1396)")
        .execute(&pool)
        .await
        .unwrap();

    let child = || async {
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "parallel_processes_own_distinct_databases_and_preserve_control_history",
            "--nocapture",
        ]);
        command.env("ISSUE_1396_ISOLATION_CHILD", "1");
        command.output().await
    };
    let (first, second) = tokio::join!(child(), child());
    let mut names = vec![database];
    for output in [first.unwrap(), second.unwrap()] {
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let name = stdout
            .lines()
            .find_map(|line| {
                line.split_once("ISSUE_1396_DATABASE=")
                    .map(|(_, name)| name)
            })
            .unwrap();
        assert!(!names.iter().any(|previous| previous == name));
        names.push(name.to_owned());
    }
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT value FROM isolation_sentinel")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1396
    );
    let control_history_after = history_snapshot(&control).await;
    assert_eq!(control_history_before, control_history_after);
}

#[tokio::test]
#[serial_test::serial]
#[should_panic(expected = "refusing mutation of an unrelated database")]
async fn cleaning_rejects_the_control_database() {
    let _isolated = common::setup_pool().await;
    let url = std::env::var("TEST_DATABASE_ADMIN_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap();
    let control = PgPool::connect(&url).await.unwrap();
    common::clean_db(&control).await;
}

#[tokio::test]
#[serial_test::serial]
async fn migration_grants_exclude_runtime_ddl_and_history_and_qa_requires_marker() {
    use cl8y_dex_indexer::database_security::{migrate_and_grant, verify_qa, verify_runtime};
    let pool = common::setup_pool().await;
    migrate_and_grant(&pool, "dex_indexer_runtime")
        .await
        .unwrap();
    let (schema_create, ledger_access, schema_version_write, app_write): (bool, bool, bool, bool) =
        sqlx::query_as(
            "SELECT has_schema_privilege('dex_indexer_runtime', 'public', 'CREATE'),
             has_table_privilege('dex_indexer_runtime', 'public._sqlx_migrations', 'SELECT,INSERT,UPDATE,DELETE'),
             has_table_privilege('dex_indexer_runtime', 'public.indexer_schema_version', 'INSERT,UPDATE,DELETE'),
             has_table_privilege('dex_indexer_runtime', 'public.assets', 'SELECT,INSERT,UPDATE,DELETE')",
        ).fetch_one(&pool).await.unwrap();
    assert!(!schema_create && !ledger_access && !schema_version_write);
    assert!(app_write);
    assert!(
        verify_runtime(&pool).await.is_err(),
        "QA credential cannot serve"
    );
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap();
    verify_qa(&pool, &database).await.unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {database} IS NULL"))
        .execute(&pool)
        .await
        .unwrap();
    assert!(verify_qa(&pool, &database).await.is_err());
    sqlx::query(&format!(
        "COMMENT ON DATABASE {database} IS 'cl8y-dex:isolated-qa:v1'"
    ))
    .execute(&pool)
    .await
    .unwrap();
    verify_qa(&pool, &database).await.unwrap();
}

#[tokio::test]
#[serial_test::serial]
#[ignore = "requires an explicitly provisioned disposable runtime LOGIN fixture and TEST_DATABASE_RUNTIME_PASSWORD"]
async fn runtime_credential_rejects_schema_ddl_and_detects_database_create_grants() {
    use cl8y_dex_indexer::database_security::{migrate_and_grant, verify_runtime};
    let password = std::env::var("TEST_DATABASE_RUNTIME_PASSWORD")
        .expect("explicit disposable runtime fixture credential required");
    // Only this optional authenticated runtime fixture shares one cluster login.
    // Keep its temporary CONNECT grant exclusive across verifier processes.
    use std::os::fd::AsRawFd;
    let _runtime_lock = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open("/tmp/dex1396-runtime-login-test.lock")
        .expect("open authenticated runtime fixture lock");
    assert_eq!(
        unsafe { libc::flock(_runtime_lock.as_raw_fd(), libc::LOCK_EX) },
        0
    );

    let pool = common::setup_pool().await;
    migrate_and_grant(&pool, "dex_indexer_runtime")
        .await
        .unwrap();
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(&format!(
        "GRANT CONNECT ON DATABASE {database} TO dex_indexer_runtime"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let options = pool
        .connect_options()
        .as_ref()
        .clone()
        .username("dex_indexer_runtime")
        .password(&password);
    let cleanup_pool = pool.clone();
    let cleanup_database = database.clone();
    // Capture assertion panics so even a failed fixture revokes its exact grant.
    let checks = tokio::spawn(async move {
    let runtime = PgPool::connect_with(options).await.unwrap();
    verify_runtime(&runtime).await.unwrap();
    for forbidden in [
        "CREATE SCHEMA forbidden_runtime_schema",
        "CREATE TABLE public.forbidden_runtime_table (id integer)",
        "SELECT * FROM public._sqlx_migrations",
    ] {
        assert!(sqlx::query(forbidden).execute(&runtime).await.is_err());
    }
    sqlx::query(&format!(
        "GRANT CREATE ON DATABASE {database} TO dex_indexer_runtime"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert!(verify_runtime(&runtime).await.is_err());
    sqlx::query(&format!(
        "REVOKE CREATE ON DATABASE {database} FROM dex_indexer_runtime"
    ))
    .execute(&pool)
    .await
    .unwrap();
    verify_runtime(&runtime).await.unwrap();
    sqlx::query("CREATE FUNCTION public.unsafe_test_definer() RETURNS integer LANGUAGE SQL SECURITY DEFINER AS 'SELECT 1'")
        .execute(&pool).await.unwrap();
    assert!(verify_runtime(&runtime).await.is_err());
    sqlx::query(
        "REVOKE EXECUTE ON FUNCTION public.unsafe_test_definer() FROM PUBLIC, dex_indexer_runtime",
    )
    .execute(&pool)
    .await
    .unwrap();
    verify_runtime(&runtime).await.unwrap();
    sqlx::query("CREATE SCHEMA unrelated_fixture_schema")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA unrelated_fixture_schema TO dex_indexer_runtime")
        .execute(&pool)
        .await
        .unwrap();
    assert!(verify_runtime(&runtime).await.is_err());
    sqlx::query("REVOKE USAGE ON SCHEMA unrelated_fixture_schema FROM dex_indexer_runtime")
        .execute(&pool)
        .await
        .unwrap();
    verify_runtime(&runtime).await.unwrap();
    runtime.close().await;
    }).await;
    sqlx::query(&format!(
        "REVOKE CONNECT ON DATABASE {cleanup_database} FROM dex_indexer_runtime"
    ))
    .execute(&cleanup_pool)
    .await
    .expect("revoke only this fixture's temporary runtime connection grant");
    if let Err(error) = checks {
        if error.is_panic() {
            std::panic::resume_unwind(error.into_panic());
        }
        panic!("runtime fixture checks cancelled: {error}");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn actual_qa_cli_seeds_cleans_and_rejects_unmarked_targets_without_writes() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let url = common::test_config().database_url;
    let command = |args: &[&str]| {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_cl8y-dex-indexer"));
        command
            .args(args)
            .env_clear()
            .current_dir(std::env::temp_dir())
            .env("RUN_MODE", "qa")
            .env("DATABASE_URL", &url)
            .env("QA_DATABASE_NAME", &database);
        command
    };
    let output = command(&["migrate"]).output().await.unwrap();
    assert!(
        output.status.success(),
        "standalone migrate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = command(&["seed-qa", "--check"]).output().await.unwrap();
    assert!(
        output.status.success(),
        "QA preflight failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = command(&["seed-qa", "--weeks", "1", "--swaps-per-day", "1"])
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "seed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let synthetic_count = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM swap_events WHERE tx_hash LIKE 'SEEDQA_%'",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    assert_eq!(synthetic_count().await, 7);
    sqlx::query(&format!("COMMENT ON DATABASE {database} IS NULL"))
        .execute(&pool)
        .await
        .unwrap();
    for args in [
        &["seed-qa"][..],
        &["seed-qa", "--clean"][..],
        &["seed-qa", "--check"][..],
    ] {
        let output = command(args).output().await.unwrap();
        assert!(!output.status.success());
        assert_eq!(synthetic_count().await, 7, "refusal must precede writes");
    }
    sqlx::query(&format!(
        "COMMENT ON DATABASE {database} IS 'cl8y-dex:isolated-qa:v1'"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let output = command(&["seed-qa", "--clean"]).output().await.unwrap();
    assert!(
        output.status.success(),
        "clean failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(synthetic_count().await, 0);
}
