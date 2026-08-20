//! Venus Core Pool vFDUSD redeem rate (GitLab #571).
//!
//! Polls BSC JSON-RPC `eth_call` for **`exchangeRateStored()`** on the pinned vFDUSD
//! market. Converts to **human FDUSD per 1 human vFDUSD**. Advisory only — not
//! CEX FDUSD/USD, not the UST1 window rate, not DEX `volume_usd` (X4 / P550-9).
//!
//! Prefer the view `exchangeRateStored`. Do **not** send exchangeRateCurrent
//! as a transaction (selector `0xbd6d894d` is unused).

use std::sync::Arc;
use std::time::Duration;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::db::queries::venus_vfdusd as db_venus;
use crate::indexer::pair_price_usd::{fits_numeric_38_18, ten_pow_i32};

/// Venus Core Pool vFDUSD (BSC). Pin — do not scrape the Venus app.
pub const DEFAULT_VENUS_VFDUSD_MARKET: &str = "0xC4eF4229FEc74Ccfe17B2bdeF7715fAC740BA0ba";
/// Underlying FDUSD on BSC (documentation pin; decimals still read on-chain).
pub const DEFAULT_VENUS_FDUSD_UNDERLYING: &str = "0xc5f0f7b66764F6ec8C8Dff7BA683102295E16409";

pub const SOURCE_VENUS_BSC: &str = "venus_bsc";
/// API / poller alias for the allowlisted source label.
pub const VENUS_SOURCE: &str = SOURCE_VENUS_BSC;
/// API pin for the Core Pool vToken (checksummed).
pub const VENUS_VFDUSD_VTOKEN: &str = DEFAULT_VENUS_VFDUSD_MARKET;

/// `exchangeRateStored()` — view. Never send as a state-changing tx.
const SELECTOR_EXCHANGE_RATE_STORED: &str = "0x182df0cd";
const SELECTOR_DECIMALS: &str = "0x313ce567";
const SELECTOR_UNDERLYING: &str = "0x6f307dc3";

pub type SharedVenusSnapshot = Arc<RwLock<Option<VenusVfdusdSnapshot>>>;
pub type SharedVenusVfdusd = SharedVenusSnapshot;

pub fn new_shared_snapshot() -> SharedVenusSnapshot {
    Arc::new(RwLock::new(None))
}

pub fn new_shared_venus() -> SharedVenusVfdusd {
    new_shared_snapshot()
}

#[derive(Debug, Clone)]
pub struct VenusVfdusdSnapshot {
    pub fdusd_per_vfdusd: BigDecimal,
    pub source: String,
    pub fetched_at: DateTime<Utc>,
    pub vtoken: String,
}

impl VenusVfdusdSnapshot {
    pub fn new_now(fdusd_per_vfdusd: BigDecimal) -> Self {
        Self {
            fdusd_per_vfdusd,
            source: VENUS_SOURCE.to_string(),
            fetched_at: Utc::now(),
            vtoken: VENUS_VFDUSD_VTOKEN.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VenusPollerConfig {
    pub rpc_urls: Vec<String>,
    pub vtoken: String,
    pub poll_interval_ms: u64,
}

impl VenusPollerConfig {
    pub fn from_indexer_config(config: &crate::config::Config) -> Self {
        Self {
            rpc_urls: config.bsc_rpc_urls.clone(),
            vtoken: DEFAULT_VENUS_VFDUSD_MARKET.to_string(),
            poll_interval_ms: config.venus_vfdusd_poll_interval_ms,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VenusError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("RPC error: {0}")]
    Rpc(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// Human FDUSD redeemed for **1 human vFDUSD**.
///
/// Compound/Venus: `underlying_raw = vtoken_raw * exchangeRateStored / 1e18`.
/// For one human vToken (`vtoken_raw = 10^vtoken_decimals`):
/// `human = rate / 10^(18 + underlying_decimals - vtoken_decimals)`.
pub fn fdusd_per_human_vfdusd(
    exchange_rate_stored: &BigDecimal,
    vtoken_decimals: u8,
    underlying_decimals: u8,
) -> Option<BigDecimal> {
    if *exchange_rate_stored <= BigDecimal::from(0) {
        return None;
    }
    if vtoken_decimals > 36 || underlying_decimals > 36 {
        return None;
    }
    let scale = 18i32 + i32::from(underlying_decimals) - i32::from(vtoken_decimals);
    let human = exchange_rate_stored * ten_pow_i32(-scale);
    if human <= BigDecimal::from(0) || !fits_numeric_38_18(&human) {
        return None;
    }
    Some(human)
}

pub fn parse_bsc_rpc_urls(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn normalize_evm_address(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))?;
    if hex.len() != 40 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("0x{}", hex.to_ascii_lowercase()))
}

pub async fn run_venus_vfdusd_loop(
    pool: PgPool,
    config: VenusPollerConfig,
    snapshot: SharedVenusSnapshot,
) {
    if let Ok(Some(row)) = db_venus::get_latest_rate(&pool).await {
        tracing::info!(
            "Venus vFDUSD: loaded cached redeem rate from DB: {} FDUSD/vFDUSD",
            row.fdusd_per_vfdusd
        );
        *snapshot.write().await = Some(VenusVfdusdSnapshot {
            fdusd_per_vfdusd: row.fdusd_per_vfdusd,
            source: row.source,
            fetched_at: row.fetched_at,
            vtoken: row.vtoken,
        });
    }

    if config.rpc_urls.is_empty() {
        tracing::warn!("Venus vFDUSD: BSC_RPC_URLS empty — skip poll, retain last known sample");
        loop {
            tokio::time::sleep(Duration::from_millis(config.poll_interval_ms.max(1_000))).await;
        }
    }

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Venus vFDUSD: failed to build HTTP client: {e}");
            return;
        }
    };

    let Some(vtoken) = normalize_evm_address(&config.vtoken) else {
        tracing::error!("Venus vFDUSD: invalid VENUS_VFDUSD_MARKET pin — skip poll");
        return;
    };

    let interval = Duration::from_millis(config.poll_interval_ms.max(1_000));
    let mut decimals_cache: Option<(u8, u8)> = None;

    loop {
        match poll_once(&client, &config.rpc_urls, &vtoken, &mut decimals_cache).await {
            Ok(rate) => {
                tracing::info!(
                    "Venus vFDUSD: {} FDUSD per 1 vFDUSD (source={})",
                    rate,
                    SOURCE_VENUS_BSC
                );
                if let Err(e) = db_venus::insert_rate(&pool, &rate, &vtoken, SOURCE_VENUS_BSC).await
                {
                    tracing::error!("Venus vFDUSD: failed to persist rate: {e}");
                }
                *snapshot.write().await = Some(VenusVfdusdSnapshot {
                    fdusd_per_vfdusd: rate,
                    source: SOURCE_VENUS_BSC.to_string(),
                    fetched_at: Utc::now(),
                    vtoken: vtoken.clone(),
                });
            }
            Err(e) => {
                tracing::warn!("Venus vFDUSD: poll failed (soft-fail, retain last known): {e}");
            }
        }
        tokio::time::sleep(interval).await;
    }
}

async fn poll_once(
    client: &Client,
    rpc_urls: &[String],
    vtoken: &str,
    decimals_cache: &mut Option<(u8, u8)>,
) -> Result<BigDecimal, VenusError> {
    let rate_hex =
        eth_call_failover(client, rpc_urls, vtoken, SELECTOR_EXCHANGE_RATE_STORED).await?;
    let rate = decode_uint256(&rate_hex)?;

    let (v_dec, u_dec) = match *decimals_cache {
        Some(pair) => pair,
        None => {
            let v_dec = decode_uint8(
                &eth_call_failover(client, rpc_urls, vtoken, SELECTOR_DECIMALS).await?,
            )?;
            let underlying_hex =
                eth_call_failover(client, rpc_urls, vtoken, SELECTOR_UNDERLYING).await?;
            let underlying = decode_address(&underlying_hex)?;
            let expected = normalize_evm_address(DEFAULT_VENUS_FDUSD_UNDERLYING);
            if expected.as_deref() != Some(underlying.as_str()) {
                return Err(VenusError::Parse(
                    "underlying mismatch vs pinned Core Pool FDUSD".into(),
                ));
            }
            let u_dec = decode_uint8(
                &eth_call_failover(client, rpc_urls, &underlying, SELECTOR_DECIMALS).await?,
            )?;
            *decimals_cache = Some((v_dec, u_dec));
            (v_dec, u_dec)
        }
    };

    fdusd_per_human_vfdusd(&rate, v_dec, u_dec).ok_or_else(|| {
        VenusError::Parse("Venus rate converted to non-finite / zero / overflow".into())
    })
}

async fn eth_call_failover(
    client: &Client,
    rpc_urls: &[String],
    to: &str,
    data: &str,
) -> Result<String, VenusError> {
    let mut last_err = VenusError::Rpc("no BSC RPC URLs".into());
    for url in rpc_urls {
        match eth_call(client, url, to, data).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::debug!("Venus vFDUSD: RPC endpoint failed: {e}");
                last_err = e;
            }
        }
    }
    Err(last_err)
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<String>,
    error: Option<serde_json::Value>,
}

async fn eth_call(
    client: &Client,
    rpc_url: &str,
    to: &str,
    data: &str,
) -> Result<String, VenusError> {
    // Read-only view. `to` and `data` are pinned selectors / allowlisted addresses.
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ]
    });
    let resp = client.post(rpc_url).json(&body).send().await?;
    let status = resp.status();
    if status.as_u16() == 429 {
        return Err(VenusError::Rpc("rate limited".into()));
    }
    if !status.is_success() {
        return Err(VenusError::Rpc(format!("HTTP {status}")));
    }
    let parsed: JsonRpcResponse = resp
        .json()
        .await
        .map_err(|e| VenusError::Parse(format!("BSC RPC JSON: {e}")))?;
    if let Some(_err) = parsed.error {
        return Err(VenusError::Rpc("BSC RPC error object".into()));
    }
    parsed
        .result
        .filter(|s| !s.is_empty() && s != "0x")
        .ok_or_else(|| VenusError::Rpc("empty eth_call result".into()))
}

pub fn decode_uint256(hex: &str) -> Result<BigDecimal, VenusError> {
    let digits = strip_0x(hex)?;
    if digits.len() > 64 {
        return Err(VenusError::Parse("uint256 hex longer than 32 bytes".into()));
    }
    hex_to_bigdecimal(digits)
}

pub fn decode_uint8(hex: &str) -> Result<u8, VenusError> {
    let n = decode_uint256(hex)?;
    if n > BigDecimal::from(255) || n < BigDecimal::from(0) {
        return Err(VenusError::Parse("decimals out of u8 range".into()));
    }
    n.to_string()
        .parse::<u8>()
        .map_err(|e| VenusError::Parse(format!("decimals parse: {e}")))
}

pub fn decode_address(hex: &str) -> Result<String, VenusError> {
    let digits = strip_0x(hex)?;
    if digits.len() < 40 {
        return Err(VenusError::Parse("address hex too short".into()));
    }
    let tail = &digits[digits.len() - 40..];
    normalize_evm_address(&format!("0x{tail}"))
        .ok_or_else(|| VenusError::Parse("invalid address hex".into()))
}

fn strip_0x(hex: &str) -> Result<&str, VenusError> {
    let trimmed = hex.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(VenusError::Parse("invalid hex".into()));
    }
    Ok(digits)
}

fn hex_to_bigdecimal(digits: &str) -> Result<BigDecimal, VenusError> {
    let mut acc = BigDecimal::from(0);
    let sixteen = BigDecimal::from(16);
    for c in digits.chars() {
        let d = c
            .to_digit(16)
            .ok_or_else(|| VenusError::Parse("invalid hex digit".into()))?;
        acc = acc * &sixteen + BigDecimal::from(d);
    }
    Ok(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn pin_is_core_pool_vfdusd() {
        assert_eq!(
            normalize_evm_address(DEFAULT_VENUS_VFDUSD_MARKET).unwrap(),
            "0xc4ef4229fec74ccfe17b2bdef7715fac740ba0ba"
        );
        assert_eq!(
            normalize_evm_address(DEFAULT_VENUS_FDUSD_UNDERLYING).unwrap(),
            "0xc5f0f7b66764f6ec8c8dff7ba683102295e16409"
        );
        assert_eq!(SELECTOR_EXCHANGE_RATE_STORED, "0x182df0cd");
        assert_ne!(SELECTOR_EXCHANGE_RATE_STORED, "0xbd6d894d");
    }

    #[test]
    fn decimal_fixture_eight_vtoken_eighteen_underlying() {
        // 1 human vToken (8dp) × 2e26 mantissa / 1e18 → 2e16 raw FDUSD / 1e18 = 0.02 human.
        let rate = BigDecimal::from_str("200000000000000000000000000").unwrap();
        let human = fdusd_per_human_vfdusd(&rate, 8, 18).expect("fixture");
        assert_eq!(human, BigDecimal::from_str("0.02").unwrap());
    }

    #[test]
    fn rejects_zero_nan_overflow() {
        assert!(fdusd_per_human_vfdusd(&BigDecimal::from(0), 8, 18).is_none());
        let huge = ten_pow_i32(60);
        assert!(fdusd_per_human_vfdusd(&huge, 8, 18).is_none());
        assert!(fdusd_per_human_vfdusd(&BigDecimal::from(1), 40, 18).is_none());
    }

    #[test]
    fn never_invents_one_to_one_peg() {
        // Missing rate must not become 1.0; conversion of 1e18 mantissa with equal decimals is 1
        // only when Venus actually reports that mantissa — callers still must not hardcode it.
        let one_e18 = ten_pow_i32(18);
        let equal_dec = fdusd_per_human_vfdusd(&one_e18, 8, 8).unwrap();
        assert_eq!(equal_dec, BigDecimal::from(1));
        assert!(fdusd_per_human_vfdusd(&BigDecimal::from(0), 8, 8).is_none());
    }

    #[test]
    fn decode_uint256_and_address() {
        let rate = decode_uint256("0xa56fa5b99019a5c8000000").unwrap();
        assert_eq!(
            rate,
            BigDecimal::from_str("200000000000000000000000000").unwrap()
        );
        assert_eq!(
            decode_uint8("0x0000000000000000000000000000000000000000000000000000000000000008")
                .unwrap(),
            8
        );
        assert_eq!(
            decode_address("0x000000000000000000000000c5f0f7b66764f6ec8c8dff7ba683102295e16409")
                .unwrap(),
            "0xc5f0f7b66764f6ec8c8dff7ba683102295e16409"
        );
    }

    #[test]
    fn normalize_rejects_injection() {
        assert!(normalize_evm_address("javascript:alert(1)").is_none());
        assert!(normalize_evm_address("../0xabc").is_none());
        assert!(normalize_evm_address("0xgg").is_none());
        assert!(normalize_evm_address(&format!("0x{}", "00".repeat(21))).is_none());
    }

    #[test]
    fn parse_rpc_urls_skips_empty() {
        assert!(parse_bsc_rpc_urls("").is_empty());
        assert_eq!(
            parse_bsc_rpc_urls(" https://a.example , ,https://b.example "),
            vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn eth_call_reads_exchange_rate_stored_not_current() {
        use wiremock::matchers::{body_partial_json, method};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(serde_json::json!({
                "method": "eth_call",
                "params": [{
                    "to": "0xc4ef4229fec74ccfe17b2bdef7715fac740ba0ba",
                    "data": "0x182df0cd"
                }, "latest"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": "0x00000000000000000000000000000000000000000a56fa5b99019a5c8000000"
            })))
            .mount(&server)
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let hex = eth_call(
            &client,
            &server.uri(),
            "0xc4ef4229fec74ccfe17b2bdef7715fac740ba0ba",
            SELECTOR_EXCHANGE_RATE_STORED,
        )
        .await
        .unwrap();
        let rate = decode_uint256(&hex).unwrap();
        assert_eq!(
            rate,
            BigDecimal::from_str("200000000000000000000000000").unwrap()
        );
    }

    #[tokio::test]
    async fn eth_call_rpc_error_is_soft_fail() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": { "code": -32000, "message": "rate limited" }
            })))
            .mount(&server)
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let err = eth_call(
            &client,
            &server.uri(),
            "0xc4ef4229fec74ccfe17b2bdef7715fac740ba0ba",
            SELECTOR_EXCHANGE_RATE_STORED,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, VenusError::Rpc(_)), "unexpected: {err}");
    }
}
