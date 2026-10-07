//! Listing-only translated markets (#1437). Never writes factory state or calls LCD.
//!
//! Identities are configured pins; prices and volumes are translated indexed fills.
//! Peg backing must be indexed and positive. Crosses use the highest USD-liquidity
//! exact-contract factory pool. CG collisions preserve the real ticker owner.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use bigdecimal::{BigDecimal, Zero};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use super::AppState;
use crate::db::queries::{assets::AssetRow, pairs::PairRow, swap_events::PairStats};
use crate::indexer::listing_exclude::{is_excluded_cw20, listing_excluded_cw20_binds};
use crate::indexer::pair_price_usd::{human_quote_per_base_from_reserves, ten_pow_i32};

pub const MARKET_IDS: [&str; 5] = [
    "cl8y-virtual:clunc-uluna",
    "cl8y-virtual:custc-uusd",
    "cl8y-virtual:uluna-uusd",
    "cl8y-virtual:uluna-usdt",
    "cl8y-virtual:clunc-uusd",
];

#[derive(Clone)]
enum Source {
    Peg {
        mapper: String,
        denom: String,
    },
    Pool {
        pair_id: i32,
        source_base_id: i32,
        source_base_decimals: i16,
        source_target_decimals: i16,
        reversed: bool,
    },
}

#[derive(Clone)]
pub struct VirtualMarket {
    pub id: &'static str,
    pub base: AssetRow,
    pub target: AssetRow,
    pub cg_enabled: bool,
    pub reserve_0: BigDecimal,
    pub reserve_1: BigDecimal,
    pub fee_bps: Option<i16>,
    pub created_at_block: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub liquidity_usd: Option<BigDecimal>,
    pub last_price: BigDecimal,
    pub stats: PairStats,
    source: Source,
    indexed_height: i64,
}

impl VirtualMarket {
    pub fn ticker_id(&self) -> String {
        format!("{}_{}", self.base.symbol, self.target.symbol)
    }
    pub fn base_id(&self) -> &str {
        self.base
            .contract_address
            .as_deref()
            .or(self.base.denom.as_deref())
            .unwrap_or("")
    }
    pub fn target_id(&self) -> &str {
        self.target
            .contract_address
            .as_deref()
            .or(self.target.denom.as_deref())
            .unwrap_or("")
    }
}

#[derive(Clone)]
pub struct ProjectedSwap {
    pub id: i64,
    pub market_id: String,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub sender: String,
    pub offer_is_base: bool,
    pub offer_amount: BigDecimal,
    pub return_amount: BigDecimal,
    pub reserve_0: Option<BigDecimal>,
    pub reserve_1: Option<BigDecimal>,
    pub price: BigDecimal,
}

#[derive(Clone, Default)]
pub struct SwapFilter {
    pub from_block: Option<i64>,
    pub to_block: Option<i64>,
    pub since: Option<DateTime<Utc>>,
    /// None is permitted only with a bounded block window or a 24h aggregation.
    pub limit: Option<i64>,
}

#[derive(FromRow)]
struct SourcePool {
    #[sqlx(flatten)]
    pair: PairRow,
    reserve_0: BigDecimal,
    reserve_1: BigDecimal,
    reserve_fee_bps: i16,
    liquidity_usd: Option<BigDecimal>,
}

async fn source_pool(
    pool: &PgPool,
    base: i32,
    target: i32,
) -> Result<Option<SourcePool>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.*, COALESCE(r.reserve_0,0) AS reserve_0, COALESCE(r.reserve_1,0) AS reserve_1, COALESCE(r.fee_bps,p.fee_bps,0)::smallint AS reserve_fee_bps, l.liquidity_usd
        FROM pairs p LEFT JOIN pair_reserves r ON r.pair_id=p.id
        JOIN assets a0 ON a0.id=p.asset_0_id JOIN assets a1 ON a1.id=p.asset_1_id
        LEFT JOIN pair_liquidity_usd l ON l.pair_id=p.id
        WHERE ((p.asset_0_id=$1 AND p.asset_1_id=$2) OR (p.asset_0_id=$2 AND p.asset_1_id=$1))
        AND LOWER(COALESCE(a0.contract_address,'')) <> ALL($3)
        AND LOWER(COALESCE(a1.contract_address,'')) <> ALL($3)
        ORDER BY l.liquidity_usd DESC NULLS LAST, p.contract_address LIMIT 1",
    )
    .bind(base)
    .bind(target)
    .bind(listing_excluded_cw20_binds())
    .fetch_optional(pool)
    .await
}

fn rescale(raw: &BigDecimal, from: i16, to: i16) -> BigDecimal {
    raw * ten_pow_i32(i32::from(to) - i32::from(from))
}

fn oriented_price(price: &BigDecimal, reversed: bool) -> Option<BigDecimal> {
    if price <= &BigDecimal::zero() {
        return None;
    }
    Some(if reversed {
        BigDecimal::from(1) / price
    } else {
        price.clone()
    })
}

async fn peg(
    pool: &PgPool,
    mapper: &str,
    base: &AssetRow,
    target: &AssetRow,
    id: &'static str,
    indexed_height: i64,
) -> Result<Option<VirtualMarket>, sqlx::Error> {
    let custody: Option<(BigDecimal, i64, DateTime<Utc>)> = sqlx::query_as(
        "SELECT custody_raw, block_height, block_timestamp FROM listing_mapper_custody
         WHERE mapper_address=$1 AND native_denom=$2 AND block_height <= $3 ORDER BY block_height DESC LIMIT 1",
    )
    .bind(mapper)
    .bind(target.denom.as_deref().unwrap_or(""))
    .bind(indexed_height)
    .fetch_optional(pool)
    .await?;
    let Some((reserve_1, height, time)) = custody.filter(|(r, _, _)| r > &BigDecimal::zero())
    else {
        return Ok(None);
    };
    let reserve_0 = rescale(&reserve_1, target.decimals, base.decimals);
    Ok(Some(VirtualMarket {
        id,
        base: base.clone(),
        target: target.clone(),
        cg_enabled: true,
        reserve_0,
        reserve_1,
        fee_bps: None,
        created_at_block: Some(height),
        created_at: time,
        liquidity_usd: None,
        last_price: BigDecimal::from(1),
        stats: PairStats::default(),
        indexed_height,
        source: Source::Peg {
            mapper: mapper.into(),
            denom: target.denom.clone().unwrap_or_default(),
        },
    }))
}

fn cross(
    id: &'static str,
    base: &AssetRow,
    target: &AssetRow,
    source_base: &AssetRow,
    source_target: &AssetRow,
    pool: &SourcePool,
) -> Option<VirtualMarket> {
    let reversed = pool.pair.asset_0_id != source_base.id;
    let (r_base, r_target) = if reversed {
        (&pool.reserve_1, &pool.reserve_0)
    } else {
        (&pool.reserve_0, &pool.reserve_1)
    };
    let reserve_0 = rescale(r_base, source_base.decimals, base.decimals);
    let reserve_1 = rescale(r_target, source_target.decimals, target.decimals);
    let last_price =
        human_quote_per_base_from_reserves(&reserve_0, &reserve_1, base.decimals, target.decimals)?;
    Some(VirtualMarket {
        id,
        base: base.clone(),
        target: target.clone(),
        cg_enabled: true,
        reserve_0,
        reserve_1,
        fee_bps: Some(pool.reserve_fee_bps),
        created_at_block: pool.pair.created_at_block,
        created_at: pool.pair.created_at,
        liquidity_usd: pool.liquidity_usd.clone(),
        last_price,
        stats: PairStats::default(),
        indexed_height: i64::MAX,
        source: Source::Pool {
            pair_id: pool.pair.id,
            source_base_id: source_base.id,
            source_base_decimals: source_base.decimals,
            source_target_decimals: source_target.decimals,
            reversed,
        },
    })
}

/// Publication checks are independent of CG symbol ownership, so GT retains its
/// opaque identity when an unrelated factory asset spoofs a CG ticker.
pub async fn load_markets(state: &AppState) -> Result<Vec<VirtualMarket>, sqlx::Error> {
    load_markets_inner(state, true).await
}

/// GT window admission loads only current publication state. It never scans
/// 24h stats, old prints or oracle metadata outside the requested block range.
pub async fn load_event_markets(state: &AppState) -> Result<Vec<VirtualMarket>, sqlx::Error> {
    load_markets_inner(state, false).await
}

async fn load_markets_inner(
    state: &AppState,
    include_ticker_state: bool,
) -> Result<Vec<VirtualMarket>, sqlx::Error> {
    let Some(mapper) = state
        .wrap_mapper_address
        .as_deref()
        .filter(|s| !s.is_empty())
    else {
        return Ok(vec![]);
    };
    let indexed_height = crate::db::queries::state::get_last_indexed_height(&state.pool).await?;
    let pins = vec![
        state.hub_usd.clunc_address.clone(),
        state.hub_usd.custc_address.clone(),
        state.hub_usd.usdt_address.clone(),
    ];
    let rows: Vec<AssetRow> = sqlx::query_as(
        "SELECT * FROM assets WHERE contract_address=ANY($1) OR denom IN ('uluna','uusd')",
    )
    .bind(&pins)
    .fetch_all(&state.pool)
    .await?;
    let cw20 = |pin: &str| {
        rows.iter().find(|a| {
            a.is_cw20 && a.contract_address.as_deref() == Some(pin) && !is_excluded_cw20(pin)
        })
    };
    let native = |denom: &str| {
        rows.iter()
            .find(|a| !a.is_cw20 && a.denom.as_deref() == Some(denom))
    };
    let clunc = cw20(&pins[0]);
    let custc = cw20(&pins[1]);
    let usdt = cw20(&pins[2]);
    let lunc = native("uluna");
    let ustc = native("uusd");
    let peg_lunc = match (clunc, lunc) {
        (Some(b), Some(t)) => peg(&state.pool, mapper, b, t, MARKET_IDS[0], indexed_height).await?,
        _ => None,
    };
    let peg_ustc = match (custc, ustc) {
        (Some(b), Some(t)) => peg(&state.pool, mapper, b, t, MARKET_IDS[1], indexed_height).await?,
        _ => None,
    };
    let mut markets = Vec::new();
    if let Some(m) = peg_lunc.clone() {
        markets.push(m);
    }
    if let Some(m) = peg_ustc.clone() {
        markets.push(m);
    }
    let cross_pool = match (clunc, custc) {
        (Some(b), Some(t)) => source_pool(&state.pool, b.id, t.id).await?,
        _ => None,
    };
    if let (Some(_), Some(_), Some(b), Some(t), Some(sb), Some(st), Some(p)) = (
        peg_lunc.as_ref(),
        peg_ustc.as_ref(),
        lunc,
        ustc,
        clunc,
        custc,
        cross_pool.as_ref(),
    ) {
        if let Some(m) = cross(MARKET_IDS[2], b, t, sb, st, p) {
            markets.push(m);
        }
    }
    if let (Some(_), Some(b), Some(t), Some(sb), Some(st)) =
        (peg_lunc.as_ref(), lunc, usdt, clunc, usdt)
    {
        if let Some(p) = source_pool(&state.pool, sb.id, st.id).await? {
            if let Some(m) = cross(MARKET_IDS[3], b, t, sb, st, &p) {
                markets.push(m);
            }
        }
    }
    if let (Some(_), Some(b), Some(t), Some(sb), Some(st), Some(p)) = (
        peg_ustc.as_ref(),
        clunc,
        ustc,
        clunc,
        custc,
        cross_pool.as_ref(),
    ) {
        if let Some(m) = cross(MARKET_IDS[4], b, t, sb, st, p) {
            markets.push(m);
        }
    }
    for market in &mut markets {
        market.indexed_height = indexed_height;
    }
    if !include_ticker_state {
        return Ok(markets);
    }
    // SQL aggregates summarize history without materializing an unbounded feed.
    for m in &mut markets {
        m.stats = load_stats(&state.pool, m).await?;
        let latest = load_swaps(
            state,
            std::slice::from_ref(m),
            SwapFilter {
                limit: Some(1),
                ..Default::default()
            },
        )
        .await?;
        if let Some(s) = latest.first() {
            m.last_price = s.price.clone();
        }
        let first = first_swap(&state.pool, m).await?;
        if let Some((height, time)) = first {
            m.created_at_block = Some(height);
            m.created_at = time;
        }
    }
    // Certified oracle quotes are used only for reserve USD, never for a print.
    let mut quotes = crate::indexer::protocol_tvl::ProtocolTvlQuotes::default();
    for ticker in [
        crate::indexer::oracle::OracleTicker::Lunc,
        crate::indexer::oracle::OracleTicker::Ustc,
    ] {
        if let Some(q) = crate::db::queries::oracle::get_oracle_state(&state.pool, ticker).await? {
            if q.is_fresh_at(Utc::now()) && !state.oracle_prices.invalidates(ticker, q.as_of).await
            {
                match ticker {
                    crate::indexer::oracle::OracleTicker::Lunc => quotes.lunc = q.price_usd,
                    crate::indexer::oracle::OracleTicker::Ustc => quotes.ustc = q.price_usd,
                    _ => {}
                }
            }
        }
    }
    for m in &mut markets {
        if matches!(m.source, Source::Peg { .. }) {
            let to_ref = |a: &AssetRow| crate::indexer::hub_usd::AssetRef {
                id: a.id,
                symbol: a.symbol.clone(),
                denom: a.denom.clone(),
                contract_address: a.contract_address.clone(),
                is_cw20: a.is_cw20,
                decimals: a.decimals,
            };
            let reserves = crate::indexer::hub_usd::ReservePair {
                pair_id: 0,
                pair_address: m.id.into(),
                asset_0: to_ref(&m.base),
                asset_1: to_ref(&m.target),
                reserve_0: m.reserve_0.clone(),
                reserve_1: m.reserve_1.clone(),
                snapshot_at: Utc::now(),
            };
            m.liquidity_usd = crate::indexer::protocol_tvl::protocol_pair_tvl(&reserves, &quotes);
        }
    }
    mark_cg_collisions(&state.pool, &mut markets).await?;
    Ok(markets)
}

async fn mark_cg_collisions(
    pool: &PgPool,
    markets: &mut [VirtualMarket],
) -> Result<(), sqlx::Error> {
    let tickers: Vec<String> = markets
        .iter()
        .map(|m| m.ticker_id().to_ascii_lowercase())
        .collect();
    let owned:Vec<String>=sqlx::query_scalar("SELECT LOWER(a0.symbol || '_' || a1.symbol) FROM pairs p
        JOIN assets a0 ON a0.id=p.asset_0_id JOIN assets a1 ON a1.id=p.asset_1_id
        WHERE LOWER(a0.symbol || '_' || a1.symbol)=ANY($1)
        AND LOWER(COALESCE(a0.contract_address,''))<>ALL($2) AND LOWER(COALESCE(a1.contract_address,''))<>ALL($2)")
        .bind(tickers).bind(listing_excluded_cw20_binds()).fetch_all(pool).await?;
    static LOGGED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    for m in markets {
        if owned
            .iter()
            .any(|t| t == &m.ticker_id().to_ascii_lowercase())
        {
            m.cg_enabled = false;
            if LOGGED
                .get_or_init(Default::default)
                .lock()
                .unwrap()
                .insert(m.id.into())
            {
                tracing::warn!(market=m.id,ticker=%m.ticker_id(),"factory ticker owns listing symbol collision");
            }
        }
    }
    Ok(())
}

#[derive(FromRow)]
struct RawSwap {
    id: i64,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: String,
    sender: String,
    offer_is_base: bool,
    offer_amount: BigDecimal,
    return_amount: BigDecimal,
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
    price: BigDecimal,
}

fn translate(m: &VirtualMarket, r: RawSwap) -> Option<ProjectedSwap> {
    let (base_dec, target_dec) = match &m.source {
        Source::Peg { .. } => (m.target.decimals, m.target.decimals),
        Source::Pool {
            source_base_decimals,
            source_target_decimals,
            ..
        } => (*source_base_decimals, *source_target_decimals),
    };
    let offer_amount = rescale(
        &r.offer_amount,
        if r.offer_is_base {
            base_dec
        } else {
            target_dec
        },
        if r.offer_is_base {
            m.base.decimals
        } else {
            m.target.decimals
        },
    );
    let return_amount = rescale(
        &r.return_amount,
        if r.offer_is_base {
            target_dec
        } else {
            base_dec
        },
        if r.offer_is_base {
            m.target.decimals
        } else {
            m.base.decimals
        },
    );
    if matches!(&m.source, Source::Pool { .. })
        && !matches!((&r.reserve_0, &r.reserve_1), (Some(a), Some(b)) if a > &BigDecimal::zero() && b > &BigDecimal::zero())
    {
        return None;
    }
    let reversed = matches!(&m.source, Source::Pool { reversed: true, .. });
    let price = oriented_price(&r.price, reversed)?;
    if offer_amount <= BigDecimal::zero()
        || return_amount <= BigDecimal::zero()
        || r.price <= BigDecimal::zero()
    {
        return None;
    }
    Some(ProjectedSwap {
        id: r.id,
        market_id: m.id.into(),
        block_height: r.block_height,
        block_timestamp: r.block_timestamp,
        tx_hash: r.tx_hash,
        sender: r.sender,
        offer_is_base: r.offer_is_base,
        offer_amount,
        return_amount,
        reserve_0: r.reserve_0.map(|v| rescale(&v, base_dec, m.base.decimals)),
        reserve_1: r
            .reserve_1
            .map(|v| rescale(&v, target_dec, m.target.decimals)),
        price,
    })
}

/// Constant five-source maximum; range filters and LIMIT are pushed into SQL.
pub async fn load_swaps(
    state: &AppState,
    markets: &[VirtualMarket],
    filter: SwapFilter,
) -> Result<Vec<ProjectedSwap>, sqlx::Error> {
    let mut out = Vec::new();
    for m in markets {
        let (sql, key, base_id, reversed, denom) = match &m.source {
            Source::Peg { mapper, denom } => (
                "SELECT id,block_height,block_timestamp,tx_hash,trader_address AS sender,
                direction='unwrap' AS offer_is_base,principal_raw AS offer_amount,principal_raw AS return_amount,
                custody_raw AS reserve_0,custody_raw AS reserve_1,1::numeric AS price FROM listing_wrap_principal
                WHERE mapper_address=$1 AND native_denom=$8 AND $6::int=0 AND NOT $7::bool AND principal_raw>0",
                mapper.clone(), 0, false, Some(denom.as_str())),
            Source::Pool { pair_id, source_base_id, reversed, .. } => (
                "SELECT id,block_height,block_timestamp,tx_hash,sender,
                offer_asset_id=$6 AS offer_is_base,offer_amount,return_amount,
                CASE WHEN $7 THEN reserve_1 ELSE reserve_0 END AS reserve_0,
                CASE WHEN $7 THEN reserve_0 ELSE reserve_1 END AS reserve_1,price FROM swap_events
                WHERE pair_id=$1::text::int AND $8::text IS NULL AND price>0 AND offer_amount>0 AND return_amount>0 AND reserve_0>0 AND reserve_1>0",
                pair_id.to_string(), *source_base_id, *reversed, None),
        };
        let sql=format!("{sql} AND ($2::bigint IS NULL OR block_height >= $2) AND ($3::bigint IS NULL OR block_height <= $3)
            AND ($4::timestamptz IS NULL OR block_timestamp >= $4) AND block_height <= $9 ORDER BY block_height DESC,id DESC LIMIT $5");
        let rows = sqlx::query_as::<_, RawSwap>(&sql)
            .bind(key)
            .bind(filter.from_block)
            .bind(filter.to_block)
            .bind(filter.since)
            .bind(filter.limit)
            .bind(base_id)
            .bind(reversed)
            .bind(denom)
            .bind(m.indexed_height)
            .fetch_all(&state.pool)
            .await?;
        out.extend(rows.into_iter().filter_map(|r| translate(m, r)));
    }
    out.sort_by(|a, b| {
        b.block_height
            .cmp(&a.block_height)
            .then(b.id.cmp(&a.id))
            .then(a.market_id.cmp(&b.market_id))
    });
    if let Some(limit) = filter.limit {
        out.truncate(limit.max(0) as usize);
    }
    Ok(out)
}

pub async fn load_window_swaps(
    state: &AppState,
    markets: &[VirtualMarket],
    from: i64,
    to: i64,
) -> Result<Vec<ProjectedSwap>, sqlx::Error> {
    load_swaps(
        state,
        markets,
        SwapFilter {
            from_block: Some(from),
            to_block: Some(to),
            ..Default::default()
        },
    )
    .await
}

#[derive(FromRow)]
struct StatsRow {
    volume_base: BigDecimal,
    volume_quote: BigDecimal,
    trade_count: i64,
    high: Option<BigDecimal>,
    low: Option<BigDecimal>,
    close_price: Option<BigDecimal>,
}

async fn load_stats(pool: &PgPool, m: &VirtualMarket) -> Result<PairStats, sqlx::Error> {
    let (sql, key, base_id, reversed, denom) = match &m.source {
        Source::Peg { mapper, denom } => (
            "SELECT principal_raw AS base_amount,principal_raw AS target_amount,1::numeric AS price,block_height,id FROM listing_wrap_principal
            WHERE mapper_address=$1 AND native_denom=$3 AND $2::int=0 AND principal_raw>0",
            mapper.clone(), 0, false, Some(denom.as_str())),
        Source::Pool { pair_id, source_base_id, reversed, .. } => (
            "SELECT CASE WHEN offer_asset_id=$2 THEN offer_amount ELSE return_amount END AS base_amount,
            CASE WHEN offer_asset_id=$2 THEN return_amount ELSE offer_amount END AS target_amount,
            price,block_height,id FROM swap_events
            WHERE pair_id=$1::text::int AND $3::text IS NULL AND price>0 AND offer_amount>0 AND return_amount>0 AND reserve_0>0 AND reserve_1>0",
            pair_id.to_string(), *source_base_id, *reversed, None),
    };
    let sql=format!("WITH fills AS ({sql} AND block_timestamp>=NOW()-INTERVAL '24 hours' AND block_height <= $4)
        SELECT COALESCE(SUM(base_amount),0) AS volume_base,COALESCE(SUM(target_amount),0) AS volume_quote,
        COUNT(*) AS trade_count,MAX(price) AS high,MIN(price) AS low,
        (SELECT price FROM fills ORDER BY block_height DESC,id DESC LIMIT 1) AS close_price FROM fills");
    let r = sqlx::query_as::<_, StatsRow>(&sql)
        .bind(key)
        .bind(base_id)
        .bind(denom)
        .bind(m.indexed_height)
        .fetch_one(pool)
        .await?;
    let (bd, td) = match &m.source {
        Source::Peg { .. } => (m.target.decimals, m.target.decimals),
        Source::Pool {
            source_base_decimals,
            source_target_decimals,
            ..
        } => (*source_base_decimals, *source_target_decimals),
    };
    Ok(PairStats {
        volume_base: rescale(&r.volume_base, bd, m.base.decimals),
        volume_quote: rescale(&r.volume_quote, td, m.target.decimals),
        trade_count: r.trade_count,
        high: (if reversed {
            r.low.clone()
        } else {
            r.high.clone()
        })
        .and_then(|p| oriented_price(&p, reversed)),
        low: (if reversed { r.high } else { r.low }).and_then(|p| oriented_price(&p, reversed)),
        close_price: r.close_price.and_then(|p| oriented_price(&p, reversed)),
        ..Default::default()
    })
}

async fn first_swap(
    pool: &PgPool,
    m: &VirtualMarket,
) -> Result<Option<(i64, DateTime<Utc>)>, sqlx::Error> {
    match &m.source {
        Source::Peg{mapper,denom}=>sqlx::query_as("SELECT block_height,block_timestamp FROM listing_wrap_principal
            WHERE mapper_address=$1 AND native_denom=$2 AND principal_raw>0 AND block_height <= $3 ORDER BY block_height,id LIMIT 1")
            .bind(mapper).bind(denom).bind(m.indexed_height).fetch_optional(pool).await,
        Source::Pool{pair_id,..}=>sqlx::query_as("SELECT block_height,block_timestamp FROM swap_events
            WHERE pair_id=$1 AND price>0 AND offer_amount>0 AND return_amount>0 AND reserve_0>0 AND reserve_1>0 AND block_height <= $2 ORDER BY block_height,id LIMIT 1")
            .bind(pair_id).bind(m.indexed_height).fetch_optional(pool).await,
    }
}

#[cfg(test)]
mod tests {
    use super::super::gt::decimalize;
    use super::*;
    use std::str::FromStr;
    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }
    #[test]
    fn reverse_price_inverts_once() {
        assert_eq!(oriented_price(&bd("0.5"), true), Some(bd("2")));
        assert_eq!(oriented_price(&bd("2"), false), Some(bd("2")));
        assert_eq!(oriented_price(&bd("0"), true), None);
    }
    #[test]
    fn principal_and_usdt_scaling_preserve_human_amounts() {
        assert_eq!(rescale(&bd("5000000"), 6, 18), bd("5000000000000000000"));
        assert_eq!(decimalize(&rescale(&bd("5000000"), 6, 18), 18), bd("5"));
        assert_eq!(rescale(&bd("5000000000000000000"), 18, 6), bd("5000000"));
    }
    #[test]
    fn five_stable_noncontract_ids() {
        let ids: HashSet<_> = MARKET_IDS.iter().collect();
        assert_eq!(ids.len(), 5);
        assert!(MARKET_IDS
            .iter()
            .all(|id| id.starts_with("cl8y-virtual:") && !id.starts_with("terra")));
    }
    fn asset(
        id: i32,
        symbol: &str,
        decimals: i16,
        contract: Option<&str>,
        denom: Option<&str>,
    ) -> AssetRow {
        AssetRow {
            id,
            contract_address: contract.map(Into::into),
            denom: denom.map(Into::into),
            is_cw20: contract.is_some(),
            name: symbol.into(),
            symbol: symbol.into(),
            decimals,
            logo_url: None,
            coingecko_id: None,
            cmc_id: None,
            first_seen_block: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
    fn reversed_usdt_market() -> VirtualMarket {
        let clunc = asset(1, "cLUNC", 6, Some("terra1clunc"), None);
        let usdt = asset(2, "USDT", 18, Some("terra1usdt"), None);
        let lunc = asset(3, "LUNC", 6, None, Some("uluna"));
        let pool = SourcePool {
            pair: PairRow {
                id: 4,
                contract_address: "terra1source".into(),
                asset_0_id: 2,
                asset_1_id: 1,
                lp_token: None,
                fee_bps: Some(30),
                hooks: vec![],
                created_at_block: Some(100),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            reserve_0: bd("4000000000000000000"),
            reserve_1: bd("2000000"),
            reserve_fee_bps: 30,
            liquidity_usd: None,
        };
        cross(MARKET_IDS[3], &lunc, &usdt, &clunc, &usdt, &pool).unwrap()
    }
    fn source_swap(is_sell: bool) -> RawSwap {
        RawSwap {
            id: 20,
            block_height: 101,
            block_timestamp: Utc::now(),
            tx_hash: "realhash".into(),
            sender: "maker".into(),
            offer_is_base: is_sell,
            offer_amount: if is_sell {
                bd("1000000")
            } else {
                bd("2000000000000000000")
            },
            return_amount: if is_sell {
                bd("2000000000000000000")
            } else {
                bd("1000000")
            },
            reserve_0: Some(bd("2000000")),
            reserve_1: Some(bd("4000000000000000000")),
            price: bd("0.5"),
        }
    }
    #[test]
    fn reversed_eighteen_decimal_cross_preserves_each_swap_side() {
        let market = reversed_usdt_market();
        assert_eq!(market.last_price, bd("2"));
        assert_eq!(market.reserve_0, bd("2000000"));
        assert_eq!(market.reserve_1, bd("4000000000000000000"));
        for sell in [true, false] {
            let projected = translate(&market, source_swap(sell)).unwrap();
            assert_eq!(projected.price, bd("2"));
            assert_eq!(projected.offer_is_base, sell);
            assert_eq!(projected.tx_hash, "realhash");
            assert_eq!(projected.sender, "maker");
            assert_eq!(
                decimalize(&projected.offer_amount, if sell { 6 } else { 18 }),
                bd(if sell { "1" } else { "2" })
            );
            assert_eq!(
                decimalize(&projected.return_amount, if sell { 18 } else { 6 }),
                bd(if sell { "2" } else { "1" })
            );
            assert_eq!(projected.reserve_0, Some(bd("2000000")));
        }
    }
    #[test]
    fn projection_rejects_zero_amount_price_and_missing_event_reserves() {
        let market = reversed_usdt_market();
        let mut raw = source_swap(true);
        raw.offer_amount = bd("0");
        assert!(translate(&market, raw).is_none());
        let mut raw = source_swap(true);
        raw.price = bd("0");
        assert!(translate(&market, raw).is_none());
        let mut raw = source_swap(true);
        raw.reserve_0 = None;
        assert!(translate(&market, raw).is_none());
        let mut raw = source_swap(true);
        raw.reserve_1 = Some(bd("0"));
        assert!(translate(&market, raw).is_none());
    }
}
