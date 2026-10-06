//! Actual executable receipts proving seed failures do not open a database socket (#1396).
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn execute_without_connection(
    args: &[&str],
    mode: Option<&str>,
    database: &str,
    extra: &[(&str, &str)],
) -> (bool, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let url = format!("postgres://dex_indexer_qa:synthetic-secret@{address}/{database}");
    let working_dir = std::env::temp_dir().join(format!(
        "dex1396-cli-{}-{}",
        std::process::id(),
        address.port()
    ));
    std::fs::create_dir(&working_dir).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_cl8y-dex-indexer"));
    command
        .args(args)
        .env_clear()
        .current_dir(&working_dir)
        .env("TOKIO_WORKER_THREADS", "1")
        .env("DATABASE_URL", url)
        .env("QA_DATABASE_NAME", database)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(mode) = mode {
        command.env("RUN_MODE", mode);
    }
    for (name, value) in extra {
        command.env(name, value);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("command did not reject its arguments/configuration promptly");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    std::fs::remove_dir(working_dir).unwrap();
    assert!(
        listener.accept().is_err(),
        "rejected command connected to the database"
    );
    let receipt = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        !receipt.contains("synthetic-secret"),
        "credential appeared in receipt"
    );
    (output.status.success(), receipt)
}

#[test]
fn seed_prod_unknown_and_missing_modes_never_connect() {
    for mode in [
        Some("prod"),
        Some("production"),
        Some("staging"),
        Some("DEV"),
        None,
    ] {
        for args in [
            &["seed-qa"][..],
            &["seed-qa", "--clean"][..],
            &["seed-qa", "--check"][..],
        ] {
            let (success, message) =
                execute_without_connection(args, mode, "dex_indexer_qa_fixture", &[]);
            assert!(!success);
            assert!(message.contains("explicit RUN_MODE"));
        }
    }
}

#[test]
fn help_and_invalid_arguments_are_handled_before_configuration() {
    let (success, _) =
        execute_without_connection(&["seed-qa", "--help"], Some("prod"), "production", &[]);
    assert!(success);
    for args in [
        &["seed-qa", "--invalid"][..],
        &["seed-qa", "--weeks"][..],
        &["seed-qa", "--weeks", "0"][..],
        &["seed-qa", "--swaps-per-day", "invalid"][..],
        &["seed-qa", "--clean", "--weeks", "4"][..],
        &["migrate", "--invalid"][..],
        &["invalid-command"][..],
    ] {
        assert!(!execute_without_connection(args, None, "production", &[]).0);
    }
}

#[test]
fn dev_mode_cannot_spoof_production_database_or_identity() {
    assert!(!execute_without_connection(&["seed-qa", "--clean"], Some("dev"), "production", &[]).0);
    assert!(
        !execute_without_connection(
            &["seed-qa"],
            Some("dev"),
            "dex_indexer_qa_fixture",
            &[("QA_DATABASE_NAME", "dex_indexer_test_other")]
        )
        .0
    );
}

#[test]
fn server_rejects_privileged_credentials_before_connection() {
    let (success, message) = execute_without_connection(
        &[],
        Some("dev"),
        "production",
        &[("MIGRATION_DATABASE_URL", "synthetic-privileged")],
    );
    assert!(!success);
    assert!(message.contains("privileged database credential"));
    assert!(!message.contains("synthetic-privileged"));
}
