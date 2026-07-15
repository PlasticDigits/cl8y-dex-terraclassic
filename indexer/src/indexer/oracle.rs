use std::sync::Arc;
use std::time::Duration;

use bigdecimal::BigDecimal;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::db::queries::oracle as db_oracle;

pub type SharedPrice = Arc<RwLock<Option<BigDecimal>>>;

pub fn new_shared_price() -> SharedPrice {
    Arc::new(RwLock::new(None))
}

pub async fn run_oracle_loop(pool: PgPool, poll_interval_ms: u64, latest_price: SharedPrice) {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build oracle HTTP client");

    if let Ok(Some(price)) = db_oracle::get_latest_average_price(&pool).await {
        tracing::info!("Oracle: loaded cached USTC/USD price from DB: {}", price);
        *latest_price.write().await = Some(price);
    }

    let interval = Duration::from_millis(poll_interval_ms);
    let mut tick_count: u64 = 0;

    loop {
        tick_count += 1;
        let fetch_coingecko = tick_count % 2 == 0;

        let results = fetch_all_sources(&client, fetch_coingecko).await;

        let mut prices: Vec<f64> = Vec::new();
        for (source, result) in &results {
            match result {
                Ok(price) => {
                    tracing::debug!("Oracle: {} returned ${:.8}", source, price);
                    prices.push(*price);
                    if let Err(e) = db_oracle::insert_price(&pool, &f64_to_bd(*price), source).await
                    {
                        tracing::error!("Oracle: failed to store {} price: {}", source, e);
                    }
                }
                Err(e) => match e {
                    OracleError::RateLimited => {
                        // CoinGecko free tier often 429s; KuCoin/MEXC usually still succeed.
                        tracing::debug!("Oracle: {} rate limited", source);
                    }
                    _ => {
                        tracing::warn!("Oracle: {} failed: {}", source, e);
                    }
                },
            }
        }

        if !prices.is_empty() {
            let avg = prices.iter().sum::<f64>() / prices.len() as f64;
            let avg_bd = f64_to_bd(avg);

            tracing::info!(
                "Oracle: USTC/USD avg ${:.8} from {}/{} sources",
                avg,
                prices.len(),
                results.len()
            );

            if let Err(e) = db_oracle::insert_price(&pool, &avg_bd, "average").await {
                tracing::error!("Oracle: failed to store average price: {}", e);
            }

            *latest_price.write().await = Some(avg_bd);
        } else {
            tracing::warn!("Oracle: all sources failed, retaining last known price");
        }

        tokio::time::sleep(interval).await;
    }
}

async fn fetch_all_sources(
    client: &Client,
    include_coingecko: bool,
) -> Vec<(&'static str, Result<f64, OracleError>)> {
    let kucoin = fetch_kucoin(client);
    let mexc = fetch_mexc(client);

    let (kc_res, mx_res) = tokio::join!(kucoin, mexc);

    let mut results = vec![("kucoin", kc_res), ("mexc", mx_res)];

    if include_coingecko {
        let cg_res = fetch_coingecko(client).await;
        results.push(("coingecko", cg_res));
    }

    results
}

fn f64_to_bd(val: f64) -> BigDecimal {
    use std::str::FromStr;
    BigDecimal::from_str(&format!("{:.18}", val)).unwrap_or_default()
}

#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("rate limited")]
    RateLimited,
    #[error("Parse error: {0}")]
    Parse(String),
}

// --- KuCoin ---

#[derive(Deserialize)]
struct KucoinResponse {
    code: String,
    data: Option<KucoinData>,
}

#[derive(Deserialize)]
struct KucoinData {
    price: Option<String>,
}

async fn fetch_kucoin(client: &Client) -> Result<f64, OracleError> {
    let resp: KucoinResponse = client
        .get("https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=USTC-USDT")
        .send()
        .await?
        .json()
        .await?;

    if resp.code != "200000" {
        return Err(OracleError::Parse(format!(
            "KuCoin returned code: {}",
            resp.code
        )));
    }

    resp.data
        .and_then(|d| d.price)
        .ok_or_else(|| OracleError::Parse("KuCoin: missing price field".into()))?
        .parse::<f64>()
        .map_err(|e| OracleError::Parse(format!("KuCoin: invalid price: {}", e)))
}

// --- MEXC ---

#[derive(Deserialize)]
struct MexcResponse {
    price: Option<String>,
}

async fn fetch_mexc(client: &Client) -> Result<f64, OracleError> {
    let resp: MexcResponse = client
        .get("https://api.mexc.com/api/v3/ticker/price?symbol=USTCUSDT")
        .send()
        .await?
        .json()
        .await?;

    resp.price
        .ok_or_else(|| OracleError::Parse("MEXC: missing price field".into()))?
        .parse::<f64>()
        .map_err(|e| OracleError::Parse(format!("MEXC: invalid price: {}", e)))
}

// --- CoinGecko (fallback) ---

#[derive(Deserialize)]
struct CoinGeckoResponse {
    terrausd: Option<CoinGeckoUsd>,
}

#[derive(Deserialize)]
struct CoinGeckoUsd {
    usd: Option<f64>,
}

/// CoinGecko free-tier rate-limit body (HTTP 429), e.g.
/// `{"status":{"error_code":429,"error_message":"..."}}`.
#[derive(Deserialize)]
struct CoinGeckoStatusBody {
    status: Option<CoinGeckoStatus>,
}

#[derive(Deserialize)]
struct CoinGeckoStatus {
    error_code: Option<u32>,
}

async fn fetch_coingecko(client: &Client) -> Result<f64, OracleError> {
    fetch_coingecko_url(
        client,
        "https://api.coingecko.com/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
    )
    .await
}

async fn fetch_coingecko_url(client: &Client, url: &str) -> Result<f64, OracleError> {
    let resp = client.get(url).send().await?;

    let status = resp.status();
    let body = resp.text().await.map_err(OracleError::Http)?;

    if status.as_u16() == 429 || coingecko_body_is_rate_limited(&body) {
        return Err(OracleError::RateLimited);
    }
    if !status.is_success() {
        return Err(OracleError::Parse(format!(
            "CoinGecko HTTP {status}: {}",
            body.chars().take(120).collect::<String>()
        )));
    }

    let parsed: CoinGeckoResponse = serde_json::from_str(&body)
        .map_err(|e| OracleError::Parse(format!("CoinGecko JSON: {e}")))?;

    parsed
        .terrausd
        .and_then(|t| t.usd)
        .ok_or_else(|| OracleError::Parse("CoinGecko: missing terrausd.usd field".into()))
}

fn coingecko_body_is_rate_limited(body: &str) -> bool {
    serde_json::from_str::<CoinGeckoStatusBody>(body)
        .ok()
        .and_then(|b| b.status)
        .and_then(|s| s.error_code)
        == Some(429)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_to_bd() {
        let bd = f64_to_bd(0.00512);
        assert!(bd > BigDecimal::from(0));
        let s = bd.to_string();
        assert!(s.starts_with("0.005"));
    }

    #[test]
    fn test_average_calculation() {
        let prices = vec![0.00510, 0.00512, 0.00514];
        let avg = prices.iter().sum::<f64>() / prices.len() as f64;
        assert!((avg - 0.00512).abs() < 1e-10);
    }

    #[test]
    fn test_average_single_source() {
        let prices = vec![0.00512];
        let avg = prices.iter().sum::<f64>() / prices.len() as f64;
        assert!((avg - 0.00512).abs() < 1e-10);
    }

    #[test]
    fn f64_to_bd_non_finite_defaults_to_zero() {
        let nan_bd = f64_to_bd(f64::NAN);
        assert_eq!(nan_bd, BigDecimal::from(0));
        let inf_bd = f64_to_bd(f64::INFINITY);
        assert_eq!(inf_bd, BigDecimal::from(0));
    }

    #[test]
    fn coingecko_rate_limit_body_detected() {
        let body = r#"{"status":{"error_code":429,"error_message":"You've exceeded the Rate Limit."}}"#;
        assert!(coingecko_body_is_rate_limited(body));
        assert!(!coingecko_body_is_rate_limited(r#"{"terrausd":{"usd":0.005}}"#));
    }

    #[tokio::test]
    async fn fetch_coingecko_maps_429_to_rate_limited() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .respond_with(ResponseTemplate::new(429).set_body_string(
                r#"{"status":{"error_code":429,"error_message":"rate limit"}}"#,
            ))
            .mount(&server)
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let err = fetch_coingecko_url(&client, &url).await.unwrap_err();
        assert!(matches!(err, OracleError::RateLimited), "unexpected: {err}");
    }
}
