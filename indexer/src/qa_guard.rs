//! Pre-connection and authenticated isolated-QA identity checks (#1396).
use std::net::IpAddr;
use std::str::FromStr;

use sqlx::postgres::PgConnectOptions;

/// This assertion is independent of RUN_MODE and cannot rename arbitrary databases into QA.
pub fn validate_seed_target(
    run_mode: Option<&str>,
    database_url: &str,
    qa_database_name: Option<&str>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        matches!(run_mode, Some("dev" | "qa" | "test" | "local")),
        "seed-qa requires explicit RUN_MODE=dev, qa, test or local; production and unknown modes are forbidden"
    );
    let url = reqwest::Url::parse(database_url)
        .map_err(|_| anyhow::anyhow!("seed-qa requires a valid isolated PostgreSQL URL"))?;
    anyhow::ensure!(
        matches!(url.scheme(), "postgres" | "postgresql")
            && url.query().is_none()
            && url.fragment().is_none(),
        "seed-qa requires a PostgreSQL URL without query/session overrides or fragments"
    );
    // Do not surface URL parser errors, which may contain credentials.
    // SQLx logs unknown query parameters, so reject them before invoking its parser.
    let options = PgConnectOptions::from_str(database_url)
        .map_err(|_| anyhow::anyhow!("seed-qa requires a valid isolated PostgreSQL URL"))?;
    let database = options.get_database().unwrap_or_default();
    anyhow::ensure!(
        qa_database_name == Some(database) && is_qa_database_name(database),
        "seed-qa requires QA_DATABASE_NAME matching a dex_indexer_qa or dex_indexer_test isolated database"
    );
    let host = options.get_host();
    let local = matches!(host, "localhost" | "postgres")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    anyhow::ensure!(
        local,
        "seed-qa only permits a loopback or postgres service host"
    );
    anyhow::ensure!(
        options.get_username() == "dex_indexer_qa",
        "seed-qa requires the dedicated dex_indexer_qa role"
    );
    Ok(())
}

pub fn is_qa_database_name(name: &str) -> bool {
    ["dex_indexer_qa", "dex_indexer_test"].iter().any(|prefix| {
        name == *prefix
            || name.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.starts_with('_')
                    && suffix.len() > 1
                    && suffix[1..]
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const QA_URL: &str = "postgres://dex_indexer_qa:synthetic@127.0.0.1/dex_indexer_qa_fixture";
    #[test]
    fn accepts_only_explicit_isolated_targets() {
        for mode in ["dev", "qa", "test", "local"] {
            assert!(
                validate_seed_target(Some(mode), QA_URL, Some("dex_indexer_qa_fixture")).is_ok()
            );
        }
        for mode in [
            None,
            Some("prod"),
            Some("production"),
            Some("staging"),
            Some(""),
        ] {
            assert!(validate_seed_target(mode, QA_URL, Some("dex_indexer_qa_fixture")).is_err());
        }
        for url in [
            "postgres://dex_indexer_qa:synthetic@127.0.0.1/production",
            "postgres://dex_indexer_qa:synthetic@db.example/dex_indexer_qa_fixture",
            "postgres://dex_indexer_migrator:synthetic@localhost/dex_indexer_qa_fixture",
            "postgres://dex_indexer_qa:synthetic@127.0.0.1/dex_indexer_qa_production-bypass",
        ] {
            assert!(
                validate_seed_target(Some("dev"), url, Some("dex_indexer_qa_fixture")).is_err()
            );
        }
        for parameter in [
            "?password=synthetic-secret",
            "?unknown=synthetic-secret",
            "?options[role]=postgres",
            "#ignored-fragment",
        ] {
            assert!(validate_seed_target(
                Some("dev"),
                &format!("{QA_URL}{parameter}"),
                Some("dex_indexer_qa_fixture")
            )
            .is_err());
        }
        assert!(validate_seed_target(Some("dev"), QA_URL, None).is_err());
        assert!(validate_seed_target(Some("dev"), QA_URL, Some("dex_indexer_qa_other")).is_err());
    }
    #[test]
    fn names_require_boundary_and_safe_suffix() {
        for name in [
            "dex_indexer_qa",
            "dex_indexer_test",
            "dex_indexer_test_wt123",
        ] {
            assert!(is_qa_database_name(name));
        }
        for name in [
            "prod",
            "dex_indexer_qaprod",
            "dex_indexer_test_",
            "dex_indexer_test_a/b",
        ] {
            assert!(!is_qa_database_name(name));
        }
    }
    #[test]
    fn errors_do_not_echo_credentials() {
        let message = validate_seed_target(Some("dev"), "postgres://secret-password@%bad", None)
            .unwrap_err()
            .to_string();
        assert!(!message.contains("secret-password"));
    }
}
