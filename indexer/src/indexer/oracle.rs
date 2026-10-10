//! External CEX/USD reference oracle (GitLab #515 / #550 / #579 / #580 / #571).
//!
//! Polls KuCoin / MEXC / CoinGecko for **USTC/USD**, **LUNC/USD**, and **FDUSD/USD**
//! (CEX First Digital USD). The HTTP/DB ticker path remains `vfdusd` (no silent `fdusd`
//! alias). Logs and additive JSON must say **FDUSD/USD**, never **vFDUSD/USD** — Terra
//! CW20 vFDUSD is Venus-bridged and is not this CEX print (#580). Protocol labels the FDUSD
//! snapshot **FDUSD reference price** on the vFDUSD tab (#571). These feeds are
//! advisory display/reference prices — not on-chain settlement.
//! Venus `eth_call` of `exchangeRateCurrent` is a separate poller (`venus_vfdusd.rs`). Volume USD conversion
//! stays on the **USTC** handle / P522-Q catalog (X4); do not multiply CEX FDUSD into
//! vFDUSD `volume_usd` / `price_usd`.
//!
//! CoinGecko's free API requires a descriptive `User-Agent` (**X7** / #579). The oracle
//! client identifies this crate and repo; it must not impersonate browsers or rotate UAs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::db::queries::oracle as db_oracle;

pub type SharedPrice = Arc<RwLock<Option<BigDecimal>>>;

/// #1391: a quote requires two independent feeds agreeing within 10%, with a
/// five minute maximum age. Bounds are representability/sanity limits, not pegs.
pub const ORACLE_MAX_AGE_SECS: i64 = 300;
pub const ORACLE_MIN_PRICE: f64 = 1e-12;
pub const ORACLE_MAX_PRICE: f64 = 1e6;
pub const ORACLE_QUORUM: usize = 2;
pub const ORACLE_SOURCE_POLICY: &str = "cex-quorum-v1";
const MAX_INLIER_RATIO: f64 = 1.1;

pub fn validate_price(value: f64) -> Result<f64, OracleError> {
    if value.is_finite() && (ORACLE_MIN_PRICE..=ORACLE_MAX_PRICE).contains(&value) {
        Ok(value)
    } else {
        Err(OracleError::Parse(
            "price is nonfinite, nonpositive or outside supported bounds".into(),
        ))
    }
}

/// Choose a unique largest agreeing cluster. A lone feed or ambiguous competing
/// clusters cannot certify a price; a gross outlier cannot contaminate two peers.
fn aggregate_sources(results: &[(&str, Result<f64, OracleError>)]) -> Option<(f64, Vec<String>)> {
    let mut values: Vec<_> = results
        .iter()
        .filter_map(|(source, result)| {
            result
                .as_ref()
                .ok()
                .and_then(|value| validate_price(*value).ok())
                .map(|value| (*source, value))
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    values.retain(|(source, _)| {
        matches!(*source, "kucoin" | "mexc" | "coingecko") && seen.insert(*source)
    });
    values.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut best = Vec::new();
    let mut ambiguous = false;
    for start in 0..values.len() {
        let cluster: Vec<_> = values[start..]
            .iter()
            .take_while(|(_, value)| *value / values[start].1 <= MAX_INLIER_RATIO)
            .cloned()
            .collect();
        if cluster.len() > best.len() {
            best = cluster;
            ambiguous = false;
        } else if cluster.len() == best.len() && cluster != best {
            ambiguous = true;
        }
    }
    if best.len() < ORACLE_QUORUM || ambiguous {
        return None;
    }
    let mean = best.iter().map(|(_, p)| p).sum::<f64>() / best.len() as f64;
    Some((mean, best.into_iter().map(|(s, _)| s.to_string()).collect()))
}

/// Stable descriptive User-Agent for CEX / CoinGecko polls (GitLab #579).
/// Compile-time crate version only — no env, tokens, emails, or hostnames.
pub const ORACLE_USER_AGENT: &str = concat!(
    "cl8y-dex-indexer/",
    env!("CARGO_PKG_VERSION"),
    " (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"
);

/// Log CoinGecko “User-Agent required” at error once, then debug (avoid warn spam).
static COINGECKO_MISSING_UA_LOGGED: AtomicBool = AtomicBool::new(false);

fn build_oracle_http_client() -> Client {
    Client::builder()
        .user_agent(ORACLE_USER_AGENT)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build oracle HTTP client")
}

const COINGECKO_INTERVAL: Duration = Duration::from_secs(60);
const FDUSD_INTERVAL: Duration = Duration::from_secs(30);

fn ticker_interval(ticker: OracleTicker, configured_ms: u64) -> Duration {
    if ticker == OracleTicker::Vfdusd {
        FDUSD_INTERVAL
    } else {
        Duration::from_millis(configured_ms.clamp(1_000, 10_000))
    }
}

/// One outstanding poll per ticker. Clock-based CoinGecko pacing does not
/// increase its traffic when the independently scheduled CEX cadence changes.
async fn run_ticker_schedule<F, Fut>(ticker: OracleTicker, period: Duration, mut poll: F)
where
    F: FnMut(bool) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut timer = tokio::time::interval(period);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Preserve the original default first CoinGecko poll at the second 30s tick.
    let mut next_coingecko = tokio::time::Instant::now() + FDUSD_INTERVAL;
    loop {
        timer.tick().await;
        let now = tokio::time::Instant::now();
        let coingecko = ticker == OracleTicker::Vfdusd || now >= next_coingecko;
        if coingecko {
            next_coingecko = now + COINGECKO_INTERVAL;
        }
        poll(coingecko).await;
    }
}

pub fn new_shared_price() -> SharedPrice {
    Arc::new(RwLock::new(None))
}

/// In-memory handles for each supported external ticker.
/// `vfdusd` (CEX FDUSD) writes must not overwrite USTC (volume_usd / overview
/// `ustc_price_usd` stay on `ustc`).
#[derive(Clone)]
pub struct OraclePriceHandles {
    pub ustc: SharedPrice,
    pub lunc: SharedPrice,
    pub vfdusd: SharedPrice,
    accepted_at: Arc<RwLock<[Option<DateTime<Utc>>; 3]>>,
    invalidated_at: Arc<RwLock<[Option<DateTime<Utc>>; 3]>>,
}

impl OraclePriceHandles {
    pub fn new() -> Self {
        Self {
            ustc: new_shared_price(),
            lunc: new_shared_price(),
            vfdusd: new_shared_price(),
            accepted_at: Arc::new(RwLock::new([None; 3])),
            invalidated_at: Arc::new(RwLock::new([None; 3])),
        }
    }

    async fn publish(
        &self,
        ticker: OracleTicker,
        price: Option<BigDecimal>,
        as_of: Option<DateTime<Utc>>,
    ) {
        // Hold timestamps while writing the price so expiry cannot race publication.
        let mut times = self.accepted_at.write().await;
        times[ticker as usize] = as_of;
        self.invalidated_at.write().await[ticker as usize] = if price.is_none() && as_of.is_none() {
            Some(Utc::now())
        } else {
            None
        };
        *self.for_ticker(ticker).write().await = price;
    }

    /// Reject a durable quote after this process observed a failed publication,
    /// even if DB SELECT remains available while its writes are failing.
    pub async fn invalidates(&self, ticker: OracleTicker, as_of: Option<DateTime<Utc>>) -> bool {
        self.invalidated_at.read().await[ticker as usize]
            .is_some_and(|failed_at| as_of.is_none_or(|time| time <= failed_at))
    }

    async fn expire(&self, now: DateTime<Utc>) {
        let times = self.accepted_at.read().await;
        for ticker in OracleTicker::ALL {
            if times[ticker as usize].is_none_or(|time| {
                time > now || now - time > chrono::Duration::seconds(ORACLE_MAX_AGE_SECS)
            }) {
                *self.for_ticker(ticker).write().await = None;
            }
        }
    }

    pub fn for_ticker(&self, ticker: OracleTicker) -> &SharedPrice {
        match ticker {
            OracleTicker::Ustc => &self.ustc,
            OracleTicker::Lunc => &self.lunc,
            OracleTicker::Vfdusd => &self.vfdusd,
        }
    }
}

impl Default for OraclePriceHandles {
    fn default() -> Self {
        Self::new()
    }
}

/// Supported external USD reference tickers exposed by the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OracleTicker {
    Ustc,
    Lunc,
    /// Path `vfdusd` stores CEX **FDUSD/USD** (MEXC `FDUSDUSDT`, CoinGecko
    /// `first-digital-usd`) — not USD of Terra CW20 vFDUSD (GitLab #580).
    /// Not a $1 hardcode. No `fdusd` path alias.
    Vfdusd,
}

impl OracleTicker {
    pub const ALL: [OracleTicker; 3] =
        [OracleTicker::Ustc, OracleTicker::Lunc, OracleTicker::Vfdusd];

    pub fn as_str(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "ustc",
            OracleTicker::Lunc => "lunc",
            OracleTicker::Vfdusd => "vfdusd",
        }
    }

    /// ASCII case-insensitive allowlist. No `fdusd` alias (I3). Homoglyphs / `../` → None.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ustc" => Some(OracleTicker::Ustc),
            "lunc" => Some(OracleTicker::Lunc),
            "vfdusd" => Some(OracleTicker::Vfdusd),
            _ => None,
        }
    }

    /// KuCoin spot symbol, if listed. CEX FDUSD is unlisted — skip that source (soft-fail).
    fn kucoin_symbol(self) -> Option<&'static str> {
        match self {
            OracleTicker::Ustc => Some("USTC-USDT"),
            OracleTicker::Lunc => Some("LUNC-USDT"),
            OracleTicker::Vfdusd => None,
        }
    }

    fn mexc_symbol(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "USTCUSDT",
            OracleTicker::Lunc => "LUNCUSDT",
            OracleTicker::Vfdusd => "FDUSDUSDT",
        }
    }

    fn coingecko_id(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "terrausd",
            OracleTicker::Lunc => "terra-luna",
            OracleTicker::Vfdusd => "first-digital-usd",
        }
    }

    /// Operator log / API display pair. Path ticker `vfdusd` still stores CEX **FDUSD**.
    pub fn display_name(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "USTC/USD",
            OracleTicker::Lunc => "LUNC/USD",
            OracleTicker::Vfdusd => "FDUSD/USD",
        }
    }

    /// CEX quote-asset identity (not the URL path). Path `vfdusd` → `FDUSD`.
    pub fn quote_asset(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "USTC",
            OracleTicker::Lunc => "LUNC",
            OracleTicker::Vfdusd => "FDUSD",
        }
    }
}

fn start_price_expiry(prices: OraclePriceHandles) -> tokio::task::JoinSet<()> {
    let mut workers = tokio::task::JoinSet::new();
    workers.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            prices.expire(Utc::now()).await;
        }
    });
    workers
}

pub async fn run_oracle_loop(pool: PgPool, poll_interval_ms: u64, prices: OraclePriceHandles) {
    let client = build_oracle_http_client();

    for ticker in OracleTicker::ALL {
        if let Ok(Some(state)) = db_oracle::get_oracle_state(&pool, ticker).await {
            if state.is_fresh_at(Utc::now()) {
                prices.publish(ticker, state.price_usd, state.as_of).await;
            }
        }
    }
    // Expire independently of upstream request duration and configured poll interval.
    // Dropping the caller's poller future also aborts its expiry child; fatal
    // indexing leaves no detached timer retaining these shared handles.
    let _expiry_worker = start_price_expiry(prices.clone());

    let mut workers = tokio::task::JoinSet::new();
    for ticker in OracleTicker::ALL {
        let client = client.clone();
        let pool = pool.clone();
        let prices = prices.clone();
        workers.spawn(async move {
            run_ticker_schedule(
                ticker,
                ticker_interval(ticker, poll_interval_ms),
                |coingecko| poll_ticker(&client, &pool, &prices, ticker, coingecko),
            )
            .await;
        });
    }
    // JoinSet is caller-owned: cancellation drops and aborts all three polls.
    if let Some(result) = workers.join_next().await {
        tracing::error!(failed = result.is_err(), "Oracle scheduled worker ended");
    }
}

async fn poll_ticker(
    client: &Client,
    pool: &PgPool,
    prices: &OraclePriceHandles,
    ticker: OracleTicker,
    fetch_coingecko: bool,
) {
    // Conservative original poll start; a slow peer or DB write must not make
    // earlier completed source responses appear freshly observed.
    let as_of = Utc::now();
    let results = fetch_all_sources(client, ticker, fetch_coingecko).await;

    for (source, result) in &results {
        match result {
            Ok(price) => {
                tracing::debug!(
                    "Oracle: {} {} returned ${:.8}",
                    ticker.as_str(),
                    source,
                    price
                );
                let source_price = match f64_to_bd(*price) {
                    Ok(value) => value,
                    Err(error) => {
                        tracing::warn!(ticker = ticker.as_str(), source, %error, "Rejected invalid source price");
                        continue;
                    }
                };
                if let Err(e) =
                    db_oracle::insert_price_at(pool, ticker, &source_price, source, as_of).await
                {
                    tracing::error!(
                        "Oracle: failed to store {} {} price: {}",
                        ticker.as_str(),
                        source,
                        e
                    );
                }
            }
            Err(e) => match e {
                OracleError::RateLimited => {
                    tracing::debug!("Oracle: {} {} rate limited", ticker.as_str(), source);
                }
                OracleError::MissingUserAgent => {
                    if COINGECKO_MISSING_UA_LOGGED.swap(true, Ordering::Relaxed) {
                        tracing::debug!(
                            "Oracle: {} {} CoinGecko HTTP 403: User-Agent required",
                            ticker.as_str(),
                            source
                        );
                    } else {
                        tracing::error!(
                            "Oracle: CoinGecko HTTP 403: User-Agent required — set a descriptive User-Agent on the oracle HTTP client (GitLab #579). Further misses at debug."
                        );
                    }
                }
                _ => {
                    tracing::warn!("Oracle: {} {} failed: {}", ticker.as_str(), source, e);
                }
            },
        }
    }

    let observation = aggregate_sources(&results)
        .and_then(|(price, sources)| f64_to_bd(price).ok().map(|price| (price, sources)));
    let (price, sources, quality) = match observation {
        Some((price, sources)) => (Some(price), sources, "fresh"),
        None => (
            None,
            Vec::new(),
            if results.iter().any(|(_, r)| r.is_ok()) {
                "degraded"
            } else {
                "unavailable"
            },
        ),
    };
    if quality != "fresh" {
        tracing::warn!(
            ticker = ticker.as_str(),
            quality,
            policy = ORACLE_SOURCE_POLICY,
            "Oracle quote unavailable: no unique agreeing source quorum"
        );
    }
    match db_oracle::record_observation_at(pool, ticker, price.as_ref(), &sources, quality, as_of)
        .await
    {
        Ok(as_of) => prices.publish(ticker, price, Some(as_of)).await,
        Err(error) => {
            tracing::error!(
                "Oracle {} could not persist quality: {}",
                ticker.as_str(),
                error
            );
            prices.publish(ticker, None, None).await;
            // A simpler write can still work if average history insertion failed.
            if let Err(invalidation_error) = db_oracle::invalidate_observation(pool, ticker).await {
                tracing::error!(
                    "Oracle {} could not persist invalidation: {}",
                    ticker.as_str(),
                    invalidation_error
                );
            }
        }
    }
}

async fn fetch_all_sources(
    client: &Client,
    ticker: OracleTicker,
    include_coingecko: bool,
) -> Vec<(&'static str, Result<f64, OracleError>)> {
    let kucoin = async {
        if ticker.kucoin_symbol().is_some() {
            Some(fetch_kucoin(client, ticker).await)
        } else {
            None
        }
    };
    let coingecko = async {
        if include_coingecko || ticker == OracleTicker::Vfdusd {
            Some(fetch_coingecko(client, ticker).await)
        } else {
            None
        }
    };
    // At most three source calls per ticker, eight across all supported tickers.
    // Slow CoinGecko no longer adds a second sequential 10s timeout to a poll.
    let (kc, mx, cg) = tokio::join!(kucoin, fetch_mexc(client, ticker), coingecko);
    let mut results = Vec::with_capacity(3);
    if let Some(result) = kc {
        results.push(("kucoin", result));
    }
    results.push(("mexc", mx));
    if let Some(result) = cg {
        results.push(("coingecko", result));
    }

    results
}

fn f64_to_bd(val: f64) -> Result<BigDecimal, OracleError> {
    use std::str::FromStr;
    validate_price(val)?;
    BigDecimal::from_str(&format!("{:.18}", val))
        .map_err(|_| OracleError::Parse("price conversion failed".into()))
}

#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("rate limited")]
    RateLimited,
    /// CoinGecko HTTP 403 whose body asks for a descriptive User-Agent (#579).
    /// Not a rate-limit; do not treat as success or insert a price.
    #[error("CoinGecko HTTP 403: User-Agent required")]
    MissingUserAgent,
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

async fn fetch_kucoin(client: &Client, ticker: OracleTicker) -> Result<f64, OracleError> {
    let symbol = ticker.kucoin_symbol().ok_or_else(|| {
        OracleError::Parse(format!("KuCoin: no listed symbol for {}", ticker.as_str()))
    })?;
    let url = format!(
        "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol={}",
        symbol
    );
    fetch_kucoin_url(client, &url).await
}

async fn fetch_kucoin_url(client: &Client, url: &str) -> Result<f64, OracleError> {
    let resp: KucoinResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
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
        .and_then(validate_price)
}

// --- MEXC ---

#[derive(Deserialize)]
struct MexcResponse {
    price: Option<String>,
}

async fn fetch_mexc(client: &Client, ticker: OracleTicker) -> Result<f64, OracleError> {
    let url = format!(
        "https://api.mexc.com/api/v3/ticker/price?symbol={}",
        ticker.mexc_symbol()
    );
    fetch_mexc_url(client, &url).await
}

async fn fetch_mexc_url(client: &Client, url: &str) -> Result<f64, OracleError> {
    let resp: MexcResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    resp.price
        .ok_or_else(|| OracleError::Parse("MEXC: missing price field".into()))?
        .parse::<f64>()
        .map_err(|e| OracleError::Parse(format!("MEXC: invalid price: {}", e)))
        .and_then(validate_price)
}

// --- CoinGecko (fallback) ---

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

async fn fetch_coingecko(client: &Client, ticker: OracleTicker) -> Result<f64, OracleError> {
    let id = ticker.coingecko_id();
    let url = format!("https://api.coingecko.com/api/v3/simple/price?ids={id}&vs_currencies=usd");
    fetch_coingecko_url(client, &url, id).await
}

async fn fetch_coingecko_url(
    client: &Client,
    url: &str,
    coin_id: &str,
) -> Result<f64, OracleError> {
    let resp = client.get(url).send().await?;

    let status = resp.status();
    let body = resp.text().await.map_err(OracleError::Http)?;

    if status.as_u16() == 429 || coingecko_body_is_rate_limited(&body) {
        return Err(OracleError::RateLimited);
    }
    if status.as_u16() == 403 && coingecko_body_asks_for_user_agent(&body) {
        return Err(OracleError::MissingUserAgent);
    }
    if !status.is_success() {
        return Err(OracleError::Parse(format!(
            "CoinGecko HTTP {status}: {}",
            body.chars().take(120).collect::<String>()
        )));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| OracleError::Parse(format!("CoinGecko JSON: {e}")))?;

    parsed
        .get(coin_id)
        .and_then(|v| serde_json::from_value::<CoinGeckoUsd>(v.clone()).ok())
        .and_then(|t| t.usd)
        .ok_or_else(|| OracleError::Parse(format!("CoinGecko: missing {coin_id}.usd field")))
        .and_then(validate_price)
}

fn coingecko_body_is_rate_limited(body: &str) -> bool {
    serde_json::from_str::<CoinGeckoStatusBody>(body)
        .ok()
        .and_then(|b| b.status)
        .and_then(|s| s.error_code)
        == Some(429)
}

fn coingecko_body_asks_for_user_agent(body: &str) -> bool {
    body.to_ascii_lowercase().contains("user-agent")
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn recovery1464_dropped_oracle_owner_aborts_expiry_child() {
        let prices = super::OraclePriceHandles::new();
        let retained = std::sync::Arc::downgrade(&prices.accepted_at);
        let workers = super::start_price_expiry(prices);
        tokio::task::yield_now().await;
        assert!(
            retained.upgrade().is_some(),
            "running expiry owns its price handles"
        );
        drop(workers);
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while retained.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cancelled oracle must release its forever timer and handles");
    }

    use super::*;

    #[test]
    fn invalid_prices_and_conversion_are_rejected() {
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.0,
            -0.01,
            1e-20,
            1e7,
            f64::MAX,
        ] {
            assert!(validate_price(value).is_err());
            assert!(f64_to_bd(value).is_err());
        }
        assert!(validate_price(1e-12).is_ok());
        assert!(validate_price(1e6).is_ok());
        assert!(validate_price("1e999".parse::<f64>().unwrap()).is_err());
    }

    #[test]
    fn quorum_all_outage_combinations_and_recovery() {
        let names = ["kucoin", "mexc", "coingecko"];
        for successes in 0u8..8 {
            let results: Vec<_> = names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    (
                        *name,
                        if successes & (1 << index) != 0 {
                            Ok(0.01)
                        } else {
                            Err(OracleError::RateLimited)
                        },
                    )
                })
                .collect();
            assert_eq!(
                aggregate_sources(&results).is_some(),
                successes.count_ones() >= 2
            );
        }
        let accepted = aggregate_sources(&[
            ("kucoin", Ok(0.01)),
            ("mexc", Ok(0.0101)),
            ("coingecko", Ok(999.0)),
        ])
        .unwrap();
        assert_eq!(accepted.1, vec!["kucoin", "mexc"]);
        assert!((accepted.0 - 0.01005).abs() < 1e-12);
        assert!(aggregate_sources(&[
            ("kucoin", Ok(1.0)),
            ("mexc", Ok(1.09)),
            ("coingecko", Ok(1.18))
        ])
        .is_none());
        assert!(aggregate_sources(&[("kucoin", Ok(1.0)), ("mexc", Ok(f64::NAN))]).is_none());
        assert!(aggregate_sources(&[("mexc", Ok(1.0)), ("mexc", Ok(1.0))]).is_none());
        assert!(aggregate_sources(&[("mexc", Ok(1.0)), ("spoof", Ok(1.0))]).is_none());
    }

    #[tokio::test]
    async fn failed_publication_invalidates_durable_quote_and_recovers() {
        let handles = OraclePriceHandles::new();
        let before = Utc::now();
        assert!(!handles.invalidates(OracleTicker::Ustc, Some(before)).await);
        handles.publish(OracleTicker::Ustc, None, None).await;
        assert!(handles.invalidates(OracleTicker::Ustc, Some(before)).await);
        assert!(handles.invalidates(OracleTicker::Ustc, None).await);
        handles
            .publish(
                OracleTicker::Ustc,
                Some(BigDecimal::from(1)),
                Some(Utc::now()),
            )
            .await;
        assert!(!handles.invalidates(OracleTicker::Ustc, Some(before)).await);
        // A persisted degraded observation retains its own honest quality.
        handles
            .publish(OracleTicker::Ustc, None, Some(Utc::now()))
            .await;
        assert!(!handles.invalidates(OracleTicker::Ustc, None).await);
    }

    #[tokio::test]
    async fn expiration_and_future_replay_clear_handles() {
        let prices = OraclePriceHandles::new();
        let now = Utc::now();
        prices
            .publish(OracleTicker::Ustc, Some(BigDecimal::from(1)), Some(now))
            .await;
        prices.expire(now + chrono::Duration::seconds(300)).await;
        assert!(prices.ustc.read().await.is_some());
        prices.expire(now + chrono::Duration::seconds(301)).await;
        assert!(prices.ustc.read().await.is_none());
        prices
            .publish(
                OracleTicker::Lunc,
                Some(BigDecimal::from(1)),
                Some(now + chrono::Duration::seconds(1)),
            )
            .await;
        prices.expire(now).await;
        assert!(prices.lunc.read().await.is_none());
    }

    #[tokio::test]
    async fn all_cex_parsers_reject_invalid_and_http_error_payloads() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let healthy = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/kucoin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"code":"200000", "data":{"price":"0.01"}})),
            )
            .mount(&healthy)
            .await;
        Mock::given(method("GET"))
            .and(path("/mexc"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"price":"0.0101"})),
            )
            .mount(&healthy)
            .await;
        assert_eq!(
            fetch_kucoin_url(
                &build_oracle_http_client(),
                &format!("{}/kucoin", healthy.uri())
            )
            .await
            .unwrap(),
            0.01
        );
        assert_eq!(
            fetch_mexc_url(
                &build_oracle_http_client(),
                &format!("{}/mexc", healthy.uri())
            )
            .await
            .unwrap(),
            0.0101
        );
        for invalid in ["NaN", "Infinity", "1e999", "-1", "0", "1000001"] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/kucoin"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"code":"200000", "data":{"price":invalid}}),
                    ),
                )
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/mexc"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"price":invalid})),
                )
                .mount(&server)
                .await;
            let client = build_oracle_http_client();
            assert!(
                fetch_kucoin_url(&client, &format!("{}/kucoin", server.uri()))
                    .await
                    .is_err()
            );
            assert!(fetch_mexc_url(&client, &format!("{}/mexc", server.uri()))
                .await
                .is_err());
        }
        for invalid in ["-1", "0", "1e999", "1000001"] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/cg"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(format!(r#"{{"terrausd":{{"usd":{invalid}}}}}"#)),
                )
                .mount(&server)
                .await;
            assert!(fetch_coingecko_url(
                &build_oracle_http_client(),
                &format!("{}/cg", server.uri()),
                "terrausd"
            )
            .await
            .is_err());
        }
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/mexc"))
            .respond_with(
                ResponseTemplate::new(500).set_body_json(serde_json::json!({"price":"1"})),
            )
            .mount(&server)
            .await;
        assert!(fetch_mexc_url(
            &build_oracle_http_client(),
            &format!("{}/mexc", server.uri())
        )
        .await
        .is_err());
    }

    #[test]
    fn ticker_parse_accepts_known() {
        assert_eq!(OracleTicker::parse("ustc"), Some(OracleTicker::Ustc));
        assert_eq!(OracleTicker::parse("LUNC"), Some(OracleTicker::Lunc));
        assert_eq!(OracleTicker::parse("VFDUSD"), Some(OracleTicker::Vfdusd));
        assert_eq!(OracleTicker::parse("unknown"), None);
        assert_eq!(OracleTicker::parse("fdusd"), None, "no silent fdusd alias");
        assert_eq!(OracleTicker::parse("btc"), None);
        assert_eq!(OracleTicker::parse("../ustc"), None);
        assert_eq!(OracleTicker::parse("javascript:ustc"), None);
        assert_eq!(OracleTicker::ALL.len(), 3);
    }

    #[test]
    fn ticker_symbols_are_distinct() {
        assert_ne!(
            OracleTicker::Ustc.kucoin_symbol(),
            OracleTicker::Lunc.kucoin_symbol()
        );
        assert_ne!(
            OracleTicker::Ustc.mexc_symbol(),
            OracleTicker::Lunc.mexc_symbol()
        );
        assert_ne!(
            OracleTicker::Ustc.coingecko_id(),
            OracleTicker::Lunc.coingecko_id()
        );
        assert_ne!(
            OracleTicker::Vfdusd.mexc_symbol(),
            OracleTicker::Ustc.mexc_symbol()
        );
        assert_ne!(
            OracleTicker::Vfdusd.mexc_symbol(),
            OracleTicker::Lunc.mexc_symbol()
        );
        assert_ne!(
            OracleTicker::Vfdusd.coingecko_id(),
            OracleTicker::Ustc.coingecko_id()
        );
        assert_ne!(
            OracleTicker::Vfdusd.coingecko_id(),
            OracleTicker::Lunc.coingecko_id()
        );
    }

    #[test]
    fn lunc_symbols_match_cex_convention() {
        assert_eq!(OracleTicker::Lunc.kucoin_symbol(), Some("LUNC-USDT"));
        assert_eq!(OracleTicker::Lunc.mexc_symbol(), "LUNCUSDT");
        assert_eq!(OracleTicker::Lunc.coingecko_id(), "terra-luna");
    }

    #[test]
    fn ustc_symbols_unchanged() {
        assert_eq!(OracleTicker::Ustc.kucoin_symbol(), Some("USTC-USDT"));
        assert_eq!(OracleTicker::Ustc.mexc_symbol(), "USTCUSDT");
        assert_eq!(OracleTicker::Ustc.coingecko_id(), "terrausd");
    }

    #[test]
    fn vfdusd_polls_fdusd_not_ustc_or_lunc_and_not_hardcoded_peg() {
        assert_eq!(OracleTicker::Vfdusd.as_str(), "vfdusd");
        assert_eq!(OracleTicker::Vfdusd.kucoin_symbol(), None);
        assert_eq!(OracleTicker::Vfdusd.mexc_symbol(), "FDUSDUSDT");
        assert_eq!(OracleTicker::Vfdusd.coingecko_id(), "first-digital-usd");
        assert_ne!(OracleTicker::Vfdusd.mexc_symbol(), "USTCUSDT");
        assert_ne!(OracleTicker::Vfdusd.mexc_symbol(), "LUNCUSDT");
        assert_ne!(OracleTicker::Vfdusd.coingecko_id(), "terrausd");
        assert_ne!(OracleTicker::Vfdusd.coingecko_id(), "terra-luna");
    }

    #[test]
    fn vfdusd_display_name_is_cex_fdusd_not_terra_vfdusd() {
        let name = OracleTicker::Vfdusd.display_name();
        assert!(name.contains("FDUSD"), "{name}");
        assert_ne!(name, "vFDUSD/USD");
        assert_eq!(name, "FDUSD/USD");
        assert_eq!(OracleTicker::Vfdusd.quote_asset(), "FDUSD");
        assert_eq!(OracleTicker::Ustc.display_name(), "USTC/USD");
        assert_eq!(OracleTicker::Lunc.display_name(), "LUNC/USD");
        assert_eq!(OracleTicker::Ustc.quote_asset(), "USTC");
        assert_eq!(OracleTicker::Lunc.quote_asset(), "LUNC");
        assert_eq!(OracleTicker::parse("fdusd"), None, "no silent fdusd alias");
        assert_eq!(OracleTicker::parse("../vfdusd"), None);
        assert_eq!(OracleTicker::parse("javascript:vfdusd"), None);
    }

    #[test]
    fn test_f64_to_bd() {
        let bd = f64_to_bd(0.00512).unwrap();
        assert!(bd > BigDecimal::from(0));
        let s = bd.to_string();
        assert!(s.starts_with("0.005"));
    }

    #[test]
    fn f64_to_bd_rejects_non_finite() {
        let nan_bd = f64_to_bd(f64::NAN);
        assert!(nan_bd.is_err());
        let inf_bd = f64_to_bd(f64::INFINITY);
        assert!(inf_bd.is_err());
    }

    #[test]
    fn coingecko_rate_limit_body_detected() {
        let body =
            r#"{"status":{"error_code":429,"error_message":"You've exceeded the Rate Limit."}}"#;
        assert!(coingecko_body_is_rate_limited(body));
        assert!(!coingecko_body_is_rate_limited(
            r#"{"terrausd":{"usd":0.005}}"#
        ));
        assert!(!coingecko_body_is_rate_limited(
            r#"{"status":{"error_code":403,"error_message":"Please add a descriptive User-Agent"}}"#
        ));
    }

    #[test]
    fn coingecko_user_agent_body_detected() {
        let ua_body = r#"{"status":{"error_code":403,"error_message":"Please add a descriptive User-Agent to your request."}}"#;
        assert!(coingecko_body_asks_for_user_agent(ua_body));
        assert!(!coingecko_body_is_rate_limited(ua_body));
        assert!(!coingecko_body_asks_for_user_agent(
            r#"{"status":{"error_code":403,"error_message":"IP banned"}}"#
        ));
        assert!(!coingecko_body_asks_for_user_agent(
            r#"{"terrausd":{"usd":0.005}}"#
        ));
    }

    #[test]
    fn oracle_user_agent_is_descriptive_not_browser() {
        let ua = ORACLE_USER_AGENT;
        assert!(ua.starts_with("cl8y-dex-indexer/"), "{ua}");
        assert!(ua.contains(env!("CARGO_PKG_VERSION")), "{ua}");
        assert!(
            ua.contains("https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic"),
            "{ua}"
        );
        let lower = ua.to_ascii_lowercase();
        assert!(!lower.contains("mozilla"), "{ua}");
        assert!(!lower.contains("chrome"), "{ua}");
        assert!(!lower.contains("firefox"), "{ua}");
        assert!(!lower.contains("keplr"), "{ua}");
        assert!(!ua.contains('@'), "no emails in UA: {ua}");
        assert!(!ua.contains("GITLAB_TOKEN"), "{ua}");
        if let Ok(token) = std::env::var("GITLAB_TOKEN") {
            if !token.is_empty() {
                assert!(!ua.contains(&token), "secret must not appear in UA");
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn recovery1464_oracle_clock_cadence_preserves_coingecko_and_fdusd_limits() {
        use std::sync::Mutex;
        let events = Arc::new(Mutex::new(Vec::new()));
        let start = tokio::time::Instant::now();
        let mut workers = tokio::task::JoinSet::new();
        for ticker in OracleTicker::ALL {
            let events = events.clone();
            workers.spawn(run_ticker_schedule(
                ticker,
                ticker_interval(ticker, 30_000),
                move |cg| {
                    events
                        .lock()
                        .unwrap()
                        .push((ticker, cg, start.elapsed().as_secs()));
                    async {}
                },
            ));
        }
        for _ in 0..=120 {
            tokio::task::yield_now().await;
            tokio::time::advance(Duration::from_secs(1)).await;
        }
        workers.shutdown().await;
        let events = events.lock().unwrap();
        for ticker in [OracleTicker::Ustc, OracleTicker::Lunc] {
            assert_eq!(
                events
                    .iter()
                    .filter(|(t, cg, _)| *t == ticker && *cg)
                    .map(|(_, _, at)| *at)
                    .collect::<Vec<_>>(),
                vec![30, 90]
            );
            assert!(events
                .iter()
                .filter(|(t, _, _)| *t == ticker)
                .map(|(_, _, at)| *at)
                .collect::<Vec<_>>()
                .windows(2)
                .all(|times| times[1] - times[0] == 10));
        }
        assert_eq!(
            events
                .iter()
                .filter(|(t, _, _)| *t == OracleTicker::Vfdusd)
                .map(|(_, cg, at)| {
                    assert!(*cg);
                    *at
                })
                .collect::<Vec<_>>(),
            vec![0, 30, 60, 90, 120]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn recovery1464_slow_ticker_does_not_delay_peers_or_overlap_and_drop_cancels() {
        use std::sync::atomic::AtomicUsize;
        let active = Arc::new(AtomicUsize::new(0));
        let normal = Arc::new(AtomicUsize::new(0));
        struct Guard(Arc<AtomicUsize>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let mut workers = tokio::task::JoinSet::new();
        let slow = active.clone();
        workers.spawn(run_ticker_schedule(
            OracleTicker::Vfdusd,
            FDUSD_INTERVAL,
            move |_| {
                assert_eq!(
                    slow.fetch_add(1, Ordering::SeqCst),
                    0,
                    "one outstanding job per ticker"
                );
                let guard = Guard(slow.clone());
                async move {
                    let _guard = guard;
                    tokio::time::sleep(Duration::from_secs(50)).await;
                }
            },
        ));
        for ticker in [OracleTicker::Ustc, OracleTicker::Lunc] {
            let count = normal.clone();
            workers.spawn(run_ticker_schedule(
                ticker,
                ticker_interval(ticker, 30_000),
                move |_| {
                    count.fetch_add(1, Ordering::SeqCst);
                    async {}
                },
            ));
        }
        for _ in 0..21 {
            tokio::task::yield_now().await;
            tokio::time::advance(Duration::from_secs(1)).await;
        }
        assert_eq!(
            normal.load(Ordering::SeqCst),
            6,
            "both healthy tickers publish at0/10/20 while peer waits"
        );
        assert_eq!(active.load(Ordering::SeqCst), 1);
        workers.shutdown().await;
        assert_eq!(
            active.load(Ordering::SeqCst),
            0,
            "owned cancellation drops in-flight poll resources"
        );
    }

    #[tokio::test]
    async fn fetch_coingecko_sends_descriptive_user_agent() {
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(query_param("ids", "terrausd"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"terrausd":{"usd":0.00512}}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let price = fetch_coingecko_url(&build_oracle_http_client(), &url, "terrausd")
            .await
            .unwrap();
        assert!((price - 0.00512).abs() < 1e-12);
    }

    #[tokio::test]
    async fn fetch_coingecko_maps_429_to_rate_limited() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(
                ResponseTemplate::new(429).set_body_string(
                    r#"{"status":{"error_code":429,"error_message":"rate limit"}}"#,
                ),
            )
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let err = fetch_coingecko_url(&build_oracle_http_client(), &url, "terrausd")
            .await
            .unwrap_err();
        assert!(matches!(err, OracleError::RateLimited), "unexpected: {err}");
        assert!(!matches!(err, OracleError::MissingUserAgent));
    }

    #[tokio::test]
    async fn fetch_coingecko_maps_403_user_agent_not_rate_limited() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(ResponseTemplate::new(403).set_body_string(
                r#"{"status":{"error_code":403,"error_message":"Please add a descriptive User-Agent to your request. For higher rate limits"},"terrausd":{"usd":1.0}}"#,
            ))
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let err = fetch_coingecko_url(&build_oracle_http_client(), &url, "terrausd")
            .await
            .unwrap_err();
        assert!(
            matches!(err, OracleError::MissingUserAgent),
            "unexpected: {err}"
        );
        assert!(!matches!(err, OracleError::RateLimited));
        assert_eq!(err.to_string(), "CoinGecko HTTP 403: User-Agent required");
        assert!(!err.to_string().contains("usd"));
    }

    #[tokio::test]
    async fn fetch_coingecko_403_other_is_parse_not_rate_limited() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let pad = "x".repeat(400);
        let body =
            format!(r#"{{"status":{{"error_code":403,"error_message":"IP banned {pad}"}}}}"#);

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(ResponseTemplate::new(403).set_body_string(&body))
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let err = fetch_coingecko_url(&build_oracle_http_client(), &url, "terrausd")
            .await
            .unwrap_err();
        match err {
            OracleError::Parse(msg) => {
                assert!(msg.contains("403"), "{msg}");
                assert!(msg.len() < body.len(), "body must be truncated: {msg}");
                assert!(!msg.contains(&"x".repeat(200)), "truncated: {msg}");
            }
            other => panic!("expected Parse, got {other}"),
        }
    }

    #[tokio::test]
    async fn fetch_coingecko_500_is_parse_not_success() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal"))
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let err = fetch_coingecko_url(&build_oracle_http_client(), &url, "terrausd")
            .await
            .unwrap_err();
        assert!(
            matches!(err, OracleError::Parse(ref s) if s.contains("500")),
            "unexpected: {err}"
        );
        assert!(!matches!(
            err,
            OracleError::RateLimited | OracleError::MissingUserAgent
        ));
    }

    #[tokio::test]
    async fn fetch_coingecko_missing_usd_is_parse() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"terrausd":{}}"#))
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let err = fetch_coingecko_url(&build_oracle_http_client(), &url, "terrausd")
            .await
            .unwrap_err();
        assert!(
            matches!(err, OracleError::Parse(ref s) if s.contains("usd")),
            "unexpected: {err}"
        );
    }

    #[tokio::test]
    async fn fetch_coingecko_parses_lunc_id() {
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(query_param("ids", "terra-luna"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"terra-luna":{"usd":0.00005024}}"#),
            )
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=terra-luna&vs_currencies=usd",
            server.uri()
        );
        let price = fetch_coingecko_url(&build_oracle_http_client(), &url, "terra-luna")
            .await
            .unwrap();
        assert!((price - 0.00005024).abs() < 1e-12);
    }

    #[tokio::test]
    async fn fetch_coingecko_parses_vfdusd_id() {
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(query_param("ids", "first-digital-usd"))
            .and(header("user-agent", ORACLE_USER_AGENT))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"first-digital-usd":{"usd":0.87}}"#),
            )
            .mount(&server)
            .await;

        let url = format!(
            "{}/api/v3/simple/price?ids=first-digital-usd&vs_currencies=usd",
            server.uri()
        );
        let price = fetch_coingecko_url(&build_oracle_http_client(), &url, "first-digital-usd")
            .await
            .unwrap();
        assert!((price - 0.87).abs() < 1e-12, "depeg must display, not $1");
    }
}
