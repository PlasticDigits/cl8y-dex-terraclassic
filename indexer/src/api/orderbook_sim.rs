use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::lcd::{LcdClient, PoolResponse};

const ORDERBOOK_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct OrderbookData {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

/// In-memory cache of simulated orderbooks to limit LCD amplification (per pair + depth).
#[derive(Clone, Default)]
pub struct OrderbookCache {
    inner: Arc<RwLock<HashMap<String, (OrderbookData, Instant)>>>,
}

/// Simulate an AMM orderbook by walking the constant-product curve.
///
/// Generates `depth` bid/ask levels from 0.1% to 10% of reserves.
/// Bids = selling asset_0 into the pool (price decreases with size).
/// Asks = buying asset_0 from the pool (price increases with size).
pub async fn simulate_orderbook(
    _pool: &PgPool,
    lcd: &LcdClient,
    pair_addr: &str,
    depth: usize,
) -> Result<OrderbookData, Box<dyn std::error::Error + Send + Sync>> {
    let pool_resp: PoolResponse = lcd
        .query_contract(pair_addr, &serde_json::json!({"pool": {}}))
        .await?;

    let reserve_0: f64 = pool_resp.assets[0].amount.parse().unwrap_or(0.0);
    let reserve_1: f64 = pool_resp.assets[1].amount.parse().unwrap_or(0.0);

    if reserve_0 <= 0.0 || reserve_1 <= 0.0 {
        return Ok(OrderbookData {
            bids: Vec::new(),
            asks: Vec::new(),
        });
    }

    let k = reserve_0 * reserve_1;
    let depth = depth.max(1);
    let divisor = (depth.max(2) - 1) as f64;

    let mut bids = Vec::with_capacity(depth);
    let mut asks = Vec::with_capacity(depth);

    for i in 0..depth {
        let fraction = 0.001 + (0.099 * (i as f64) / divisor);

        // Bid: sell dx of asset_0 into the pool, receive dy of asset_1
        let dx_bid = reserve_0 * fraction;
        let new_y = k / (reserve_0 + dx_bid);
        let dy_bid = reserve_1 - new_y;
        let bid_price = dy_bid / dx_bid;
        bids.push([format!("{:.18}", bid_price), format!("{:.18}", dx_bid)]);

        // Ask: buy dx of asset_0 from the pool, pay dy of asset_1
        let dx_ask = reserve_0 * fraction;
        if dx_ask < reserve_0 {
            let new_y_ask = k / (reserve_0 - dx_ask);
            let dy_ask = new_y_ask - reserve_1;
            let ask_price = dy_ask / dx_ask;
            asks.push([format!("{:.18}", ask_price), format!("{:.18}", dx_ask)]);
        }
    }

    Ok(OrderbookData { bids, asks })
}

/// Like [`simulate_orderbook`] but caches results per `(pair_addr, depth)` for [`ORDERBOOK_CACHE_TTL`].
pub async fn simulate_orderbook_cached(
    cache: &OrderbookCache,
    pool: &PgPool,
    lcd: &LcdClient,
    pair_addr: &str,
    depth: usize,
) -> Result<OrderbookData, Box<dyn std::error::Error + Send + Sync>> {
    let key = format!("{}:{}", pair_addr, depth);
    let now = Instant::now();
    {
        let guard = cache.inner.read().await;
        if let Some((data, exp)) = guard.get(&key) {
            if now < *exp {
                return Ok(data.clone());
            }
        }
    }

    let data = simulate_orderbook(pool, lcd, pair_addr, depth).await?;
    let mut guard = cache.inner.write().await;
    guard.insert(key, (data.clone(), now + ORDERBOOK_CACHE_TTL));
    Ok(data)
}
