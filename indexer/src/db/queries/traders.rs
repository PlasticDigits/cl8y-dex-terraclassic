use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct TraderRow {
    pub id: i32,
    pub address: String,
    pub total_trades: i64,
    pub total_volume: BigDecimal,
    /// P522-Q USD (`SUM(swap_events.volume_usd)`). NULL when the trader has no priced swaps (#553).
    pub total_volume_usd: Option<BigDecimal>,
    pub volume_24h: BigDecimal,
    pub volume_7d: BigDecimal,
    pub volume_30d: BigDecimal,
    pub tier_id: i16,
    pub tier_name: String,
    pub registered: bool,
    pub first_trade_at: Option<DateTime<Utc>>,
    pub last_trade_at: Option<DateTime<Utc>>,
    pub total_realized_pnl: BigDecimal,
    pub best_trade_pnl: Option<BigDecimal>,
    pub worst_trade_pnl: Option<BigDecimal>,
    pub total_fees_paid: BigDecimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn upsert_trader(
    pool: &PgPool,
    address: &str,
    trade_volume: &BigDecimal,
    trade_volume_usd: Option<&BigDecimal>,
) -> Result<bool, sqlx::Error> {
    let usd = match trade_volume_usd {
        Some(v) if v > &BigDecimal::from(0) => Some(v.clone()),
        _ => None,
    };
    let row = sqlx::query_scalar::<_, bool>(
        "INSERT INTO traders (address, total_trades, total_volume, total_volume_usd, first_trade_at, last_trade_at)
         VALUES ($1, 1, $2, $3, NOW(), NOW())
         ON CONFLICT (address)
           DO UPDATE SET total_trades = traders.total_trades + 1,
                        total_volume = traders.total_volume + $2,
                        total_volume_usd = CASE
                          WHEN $3::numeric IS NULL THEN traders.total_volume_usd
                          ELSE LEAST(
                            COALESCE(traders.total_volume_usd, 0) + $3,
                            POWER(10::numeric, 20) - POWER(10::numeric, -18)
                          )
                        END,
                        first_trade_at = COALESCE(traders.first_trade_at, EXCLUDED.first_trade_at),
                        last_trade_at = NOW(),
                        updated_at = NOW()
         RETURNING (xmax = 0) AS inserted",
    )
    .bind(address)
    .bind(trade_volume)
    .bind(usd)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Insert or update tier fields for a trader (registration events or LCD hydrate).
pub async fn upsert_trader_tier(
    pool: &PgPool,
    address: &str,
    tier_id: i16,
    tier_name: &str,
    registered: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO traders (address, tier_id, tier_name, registered, total_trades, total_volume)
         VALUES ($1, $2, $3, $4, 0, 0)
         ON CONFLICT (address)
           DO UPDATE SET tier_id = EXCLUDED.tier_id,
                        tier_name = EXCLUDED.tier_name,
                        registered = EXCLUDED.registered,
                        updated_at = NOW()",
    )
    .bind(address)
    .bind(tier_id)
    .bind(tier_name)
    .bind(registered)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_trader(pool: &PgPool, address: &str) -> Result<Option<TraderRow>, sqlx::Error> {
    sqlx::query_as::<_, TraderRow>("SELECT * FROM traders WHERE address = $1")
        .bind(address)
        .fetch_optional(pool)
        .await
}

pub async fn get_leaderboard(
    pool: &PgPool,
    sort_by: &str,
    limit: i64,
) -> Result<Vec<TraderRow>, sqlx::Error> {
    // Allowlisted identifiers only (GitLab #280 / SEC). USD sort uses NULLS LAST so unpriced
    // traders do not rank above priced volume (Postgres DESC defaults to NULLS FIRST).
    let sql = match sort_by {
        "volume_24h" => "SELECT * FROM traders ORDER BY volume_24h DESC LIMIT $1",
        "volume_7d" => "SELECT * FROM traders ORDER BY volume_7d DESC LIMIT $1",
        "volume_30d" => "SELECT * FROM traders ORDER BY volume_30d DESC LIMIT $1",
        "total_trades" => "SELECT * FROM traders ORDER BY total_trades DESC LIMIT $1",
        "total_realized_pnl" => "SELECT * FROM traders ORDER BY total_realized_pnl DESC LIMIT $1",
        "best_trade_pnl" => "SELECT * FROM traders ORDER BY best_trade_pnl DESC LIMIT $1",
        "worst_trade_pnl" => "SELECT * FROM traders ORDER BY worst_trade_pnl DESC LIMIT $1",
        "total_fees_paid" => "SELECT * FROM traders ORDER BY total_fees_paid DESC LIMIT $1",
        "total_volume_usd" => {
            "SELECT * FROM traders ORDER BY total_volume_usd DESC NULLS LAST LIMIT $1"
        }
        _ => "SELECT * FROM traders ORDER BY total_volume DESC LIMIT $1",
    };

    sqlx::query_as::<_, TraderRow>(&sql)
        .bind(limit)
        .fetch_all(pool)
        .await
}

/// Pair-scoped leaderboard row (GitLab #666). Lifetime-on-this-pair volume from
/// `swap_events`; realized P&L from `trader_positions`. Rolling windows and
/// global best/worst trade extrema are **not** filled here.
#[derive(Debug, Clone, FromRow)]
pub struct PairLeaderboardRow {
    pub address: String,
    pub total_trades: i64,
    pub total_volume: BigDecimal,
    pub total_volume_usd: Option<BigDecimal>,
    pub first_trade_at: Option<DateTime<Utc>>,
    pub last_trade_at: Option<DateTime<Utc>>,
    pub total_realized_pnl: BigDecimal,
    pub tier_id: Option<i16>,
    pub tier_name: Option<String>,
    pub registered: bool,
}

/// Rank wallets on one indexed pair. `sort_by` is allowlisted by the HTTP layer
/// (`PAIR_SCOPED_SORTS`). Volume sorts `GROUP BY sender` on `swap_events` for
/// `pair_id` (uses `idx_swaps_pair_time`). P&L sorts read `trader_positions`
/// for that pair — never `traders.total_*` (all-pairs).
pub async fn get_leaderboard_for_pair(
    pool: &PgPool,
    pair_id: i32,
    sort_by: &str,
    limit: i64,
) -> Result<Vec<PairLeaderboardRow>, sqlx::Error> {
    match sort_by {
        "total_realized_pnl" | "worst_trade_pnl" => {
            leaderboard_for_pair_pnl(pool, pair_id, sort_by, limit).await
        }
        _ => leaderboard_for_pair_volume(pool, pair_id, sort_by, limit).await,
    }
}

async fn leaderboard_for_pair_volume(
    pool: &PgPool,
    pair_id: i32,
    sort_by: &str,
    limit: i64,
) -> Result<Vec<PairLeaderboardRow>, sqlx::Error> {
    let order = match sort_by {
        "total_volume_usd" => "ORDER BY SUM(se.volume_usd) DESC NULLS LAST",
        "total_trades" => "ORDER BY COUNT(*) DESC",
        _ => "ORDER BY SUM(se.offer_amount) DESC",
    };
    let sql = format!(
        "SELECT
            se.sender AS address,
            COUNT(*)::bigint AS total_trades,
            COALESCE(SUM(se.offer_amount), 0) AS total_volume,
            SUM(se.volume_usd) AS total_volume_usd,
            MIN(se.block_timestamp) AS first_trade_at,
            MAX(se.block_timestamp) AS last_trade_at,
            COALESCE(tp.realized_pnl, 0) AS total_realized_pnl,
            t.tier_id,
            t.tier_name,
            COALESCE(t.registered, false) AS registered
         FROM swap_events se
         LEFT JOIN traders t ON t.address = se.sender
         LEFT JOIN trader_positions tp
           ON tp.trader_address = se.sender AND tp.pair_id = $1
         WHERE se.pair_id = $1
         GROUP BY se.sender, tp.realized_pnl, t.tier_id, t.tier_name, t.registered
         {order}
         LIMIT $2"
    );
    sqlx::query_as::<_, PairLeaderboardRow>(&sql)
        .bind(pair_id)
        .bind(limit)
        .fetch_all(pool)
        .await
}

async fn leaderboard_for_pair_pnl(
    pool: &PgPool,
    pair_id: i32,
    sort_by: &str,
    limit: i64,
) -> Result<Vec<PairLeaderboardRow>, sqlx::Error> {
    let order = if sort_by == "worst_trade_pnl" {
        "ORDER BY tp.realized_pnl ASC"
    } else {
        "ORDER BY tp.realized_pnl DESC"
    };
    let sql = format!(
        "SELECT
            tp.trader_address AS address,
            COALESCE(vol.total_trades, tp.trade_count::bigint) AS total_trades,
            COALESCE(vol.total_volume, 0) AS total_volume,
            vol.total_volume_usd,
            vol.first_trade_at,
            vol.last_trade_at,
            tp.realized_pnl AS total_realized_pnl,
            t.tier_id,
            t.tier_name,
            COALESCE(t.registered, false) AS registered
         FROM trader_positions tp
         LEFT JOIN (
           SELECT
             sender,
             COUNT(*)::bigint AS total_trades,
             COALESCE(SUM(offer_amount), 0) AS total_volume,
             SUM(volume_usd) AS total_volume_usd,
             MIN(block_timestamp) AS first_trade_at,
             MAX(block_timestamp) AS last_trade_at
           FROM swap_events
           WHERE pair_id = $1
           GROUP BY sender
         ) vol ON vol.sender = tp.trader_address
         LEFT JOIN traders t ON t.address = tp.trader_address
         WHERE tp.pair_id = $1
         {order}
         LIMIT $2"
    );
    sqlx::query_as::<_, PairLeaderboardRow>(&sql)
        .bind(pair_id)
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn update_trader_tier(
    pool: &PgPool,
    address: &str,
    tier_id: i16,
    tier_name: &str,
    registered: bool,
) -> Result<(), sqlx::Error> {
    upsert_trader_tier(pool, address, tier_id, tier_name, registered).await?;
    Ok(())
}

pub async fn refresh_rolling_volumes(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let cutoff_24h = now - chrono::Duration::hours(24);
    let cutoff_7d = now - chrono::Duration::days(7);
    let cutoff_30d = now - chrono::Duration::days(30);

    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE traders t SET
           volume_24h = COALESCE(sub.vol_24h, 0),
           volume_7d  = COALESCE(sub.vol_7d, 0),
           volume_30d = COALESCE(sub.vol_30d, 0),
           updated_at = NOW()
         FROM (
           SELECT
             sender,
             LEAST(SUM(CASE WHEN block_timestamp >= $1 THEN offer_amount ELSE 0 END), POWER(10::numeric, 38) - 1) AS vol_24h,
             LEAST(SUM(CASE WHEN block_timestamp >= $2 THEN offer_amount ELSE 0 END), POWER(10::numeric, 38) - 1) AS vol_7d,
             LEAST(SUM(CASE WHEN block_timestamp >= $3 THEN offer_amount ELSE 0 END), POWER(10::numeric, 38) - 1) AS vol_30d
           FROM swap_events
           WHERE block_timestamp >= $3
           GROUP BY sender
         ) sub
         WHERE t.address = sub.sender",
    )
    .bind(cutoff_24h)
    .bind(cutoff_7d)
    .bind(cutoff_30d)
    .execute(&mut *tx)
    .await?;

    // Traders with no swap in the 30d window are absent from the subquery; zero rolling
    // columns only (never total_volume / total_volume_usd / total_trades — GitLab #577 **D2**).
    sqlx::query(
        "UPDATE traders t SET
           volume_24h = 0,
           volume_7d = 0,
           volume_30d = 0,
           updated_at = NOW()
         WHERE NOT EXISTS (
           SELECT 1 FROM swap_events se
           WHERE se.sender = t.address AND se.block_timestamp >= $1
         )",
    )
    .bind(cutoff_30d)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Recompute `traders.total_volume_usd` from `swap_events.volume_usd` (P522-Q, GitLab #553).
/// Idempotent. Senders with no priced swaps stay NULL (not 0).
pub async fn refresh_trader_total_volume_usd(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE traders t
         SET total_volume_usd = sub.usd,
             updated_at = NOW()
         FROM (
           SELECT
             sender,
             LEAST(
               SUM(volume_usd),
               POWER(10::numeric, 20) - POWER(10::numeric, -18)
             ) AS usd
           FROM swap_events
           WHERE volume_usd IS NOT NULL AND volume_usd > 0
           GROUP BY sender
         ) sub
         WHERE t.address = sub.sender",
    )
    .execute(pool)
    .await?;
    Ok(())
}
