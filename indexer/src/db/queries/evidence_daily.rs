//! UTC-day redacted evidence rows for `GET /api/v1/evidence/daily` (Forgejo #1205).
//!
//! Every UNION arm uses the same column aliases. Postgres names a UNION from the
//! first arm only, so a `surface=limit` / `lp` / `wrap` request (no swap arm)
//! would otherwise expose `?column?` and fail `FromRow` with a 500.
//!
//! Sort is `(block_height, tx_hash, surface, kind, ordinal, row_id)`. `row_id`
//! is the source primary key and is not part of the JSON body. It breaks ties
//! when two pair swaps in one tx share `swap_index` (unique per pair, not per tx).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSortKey {
    pub block_height: i64,
    pub tx_hash: String,
    pub surface: String,
    pub kind: String,
    pub ordinal: i64,
    pub row_id: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct EvidenceRawRow {
    pub block_height: i64,
    pub tx_hash: String,
    pub surface: String,
    pub kind: String,
    pub ordinal: i64,
    pub row_id: i64,
    pub block_timestamp: DateTime<Utc>,
    pub pair_address: Option<String>,
    pub actor: Option<String>,
    pub offer_amount: Option<sqlx::types::BigDecimal>,
    pub return_amount: Option<sqlx::types::BigDecimal>,
    pub pool_return_amount: Option<sqlx::types::BigDecimal>,
    pub book_return_amount: Option<sqlx::types::BigDecimal>,
    pub offer_decimals: Option<i16>,
    pub ask_decimals: Option<i16>,
    pub order_id: Option<i64>,
    pub side: Option<String>,
    pub price: Option<sqlx::types::BigDecimal>,
    pub token0_amount: Option<sqlx::types::BigDecimal>,
    pub token1_amount: Option<sqlx::types::BigDecimal>,
    pub commission_amount: Option<sqlx::types::BigDecimal>,
    pub swap_event_id: Option<i64>,
    pub asset_0_amount: Option<sqlx::types::BigDecimal>,
    pub asset_1_amount: Option<sqlx::types::BigDecimal>,
    pub lp_amount: Option<sqlx::types::BigDecimal>,
    pub amount_raw: Option<sqlx::types::BigDecimal>,
    pub fee_decimals: Option<i16>,
    pub token_denom: Option<String>,
    pub token_contract: Option<String>,
    pub fee_usd: Option<sqlx::types::BigDecimal>,
}

#[derive(Debug, Clone, Copy)]
pub struct EvidenceSurfaceSet {
    pub swap: bool,
    pub wrap: bool,
    pub limit: bool,
    pub lp: bool,
}

impl EvidenceSurfaceSet {
    pub fn all() -> Self {
        Self {
            swap: true,
            wrap: true,
            limit: true,
            lp: true,
        }
    }
}

fn push_window(
    qb: &mut QueryBuilder<Postgres>,
    column: &str,
    day_start: DateTime<Utc>,
    day_end: DateTime<Utc>,
) {
    qb.push(column);
    qb.push(" >= ");
    qb.push_bind(day_start);
    qb.push(" AND ");
    qb.push(column);
    qb.push(" < ");
    qb.push_bind(day_end);
}

/// One UTC calendar day `[day_start, day_end)`, optional surfaces, keyset cursor, `limit+1` rows.
pub async fn list_evidence_rows(
    pool: &PgPool,
    day_start: DateTime<Utc>,
    day_end: DateTime<Utc>,
    surfaces: EvidenceSurfaceSet,
    cursor: Option<&EvidenceSortKey>,
    limit: i64,
) -> Result<Vec<EvidenceRawRow>, sqlx::Error> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM (");
    let mut first = true;

    if surfaces.swap {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(
            r"
        SELECT s.block_height,
               s.tx_hash::text AS tx_hash,
               'swap'::text AS surface,
               'swap'::text AS kind,
               COALESCE(s.swap_index, 0)::bigint AS ordinal,
               s.id AS row_id,
               s.block_timestamp,
               p.contract_address::text AS pair_address,
               s.sender::text AS actor,
               s.offer_amount,
               s.return_amount,
               s.pool_return_amount,
               s.book_return_amount,
               a0.decimals AS offer_decimals,
               a1.decimals AS ask_decimals,
               NULL::bigint AS order_id,
               NULL::text AS side,
               NULL::numeric AS price,
               NULL::numeric AS token0_amount,
               NULL::numeric AS token1_amount,
               NULL::numeric AS commission_amount,
               NULL::bigint AS swap_event_id,
               NULL::numeric AS asset_0_amount,
               NULL::numeric AS asset_1_amount,
               NULL::numeric AS lp_amount,
               NULL::numeric AS amount_raw,
               NULL::smallint AS fee_decimals,
               NULL::text AS token_denom,
               NULL::text AS token_contract,
               NULL::numeric AS fee_usd
        FROM swap_events s
        JOIN pairs p ON p.id = s.pair_id
        JOIN assets a0 ON a0.id = s.offer_asset_id
        JOIN assets a1 ON a1.id = s.ask_asset_id
        WHERE ",
        );
        push_window(&mut qb, "s.block_timestamp", day_start, day_end);
    }

    if surfaces.lp {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(
            r"
        SELECT le.block_height,
               le.tx_hash::text AS tx_hash,
               'lp'::text AS surface,
               le.event_type::text AS kind,
               le.id AS ordinal,
               le.id AS row_id,
               le.block_timestamp,
               p.contract_address::text AS pair_address,
               le.provider::text AS actor,
               NULL::numeric AS offer_amount,
               NULL::numeric AS return_amount,
               NULL::numeric AS pool_return_amount,
               NULL::numeric AS book_return_amount,
               NULL::smallint AS offer_decimals,
               NULL::smallint AS ask_decimals,
               NULL::bigint AS order_id,
               NULL::text AS side,
               NULL::numeric AS price,
               NULL::numeric AS token0_amount,
               NULL::numeric AS token1_amount,
               NULL::numeric AS commission_amount,
               NULL::bigint AS swap_event_id,
               le.asset_0_amount,
               le.asset_1_amount,
               le.lp_amount,
               NULL::numeric AS amount_raw,
               NULL::smallint AS fee_decimals,
               NULL::text AS token_denom,
               NULL::text AS token_contract,
               NULL::numeric AS fee_usd
        FROM liquidity_events le
        JOIN pairs p ON p.id = le.pair_id
        WHERE ",
        );
        push_window(&mut qb, "le.block_timestamp", day_start, day_end);
    }

    if surfaces.limit {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(
            r"
        SELECT pl.block_height,
               pl.tx_hash::text AS tx_hash,
               'limit'::text AS surface,
               'place'::text AS kind,
               pl.id AS ordinal,
               pl.id AS row_id,
               pl.block_timestamp,
               p.contract_address::text AS pair_address,
               pl.owner::text AS actor,
               NULL::numeric AS offer_amount,
               NULL::numeric AS return_amount,
               NULL::numeric AS pool_return_amount,
               NULL::numeric AS book_return_amount,
               NULL::smallint AS offer_decimals,
               NULL::smallint AS ask_decimals,
               pl.order_id,
               pl.side::text AS side,
               pl.price,
               NULL::numeric AS token0_amount,
               NULL::numeric AS token1_amount,
               NULL::numeric AS commission_amount,
               NULL::bigint AS swap_event_id,
               NULL::numeric AS asset_0_amount,
               NULL::numeric AS asset_1_amount,
               NULL::numeric AS lp_amount,
               NULL::numeric AS amount_raw,
               NULL::smallint AS fee_decimals,
               NULL::text AS token_denom,
               NULL::text AS token_contract,
               NULL::numeric AS fee_usd
        FROM limit_order_placements pl
        JOIN pairs p ON p.id = pl.pair_id
        WHERE ",
        );
        push_window(&mut qb, "pl.block_timestamp", day_start, day_end);

        qb.push(" UNION ALL ");
        qb.push(
            r"
        SELECT c.block_height,
               c.tx_hash::text AS tx_hash,
               'limit'::text AS surface,
               'cancel'::text AS kind,
               c.id AS ordinal,
               c.id AS row_id,
               c.block_timestamp,
               p.contract_address::text AS pair_address,
               c.owner::text AS actor,
               NULL::numeric AS offer_amount,
               NULL::numeric AS return_amount,
               NULL::numeric AS pool_return_amount,
               NULL::numeric AS book_return_amount,
               NULL::smallint AS offer_decimals,
               NULL::smallint AS ask_decimals,
               c.order_id,
               NULL::text AS side,
               NULL::numeric AS price,
               NULL::numeric AS token0_amount,
               NULL::numeric AS token1_amount,
               NULL::numeric AS commission_amount,
               NULL::bigint AS swap_event_id,
               NULL::numeric AS asset_0_amount,
               NULL::numeric AS asset_1_amount,
               NULL::numeric AS lp_amount,
               NULL::numeric AS amount_raw,
               NULL::smallint AS fee_decimals,
               NULL::text AS token_denom,
               NULL::text AS token_contract,
               NULL::numeric AS fee_usd
        FROM limit_order_cancellations c
        JOIN pairs p ON p.id = c.pair_id
        WHERE ",
        );
        push_window(&mut qb, "c.block_timestamp", day_start, day_end);

        qb.push(" UNION ALL ");
        qb.push(
            r"
        SELECT f.block_height,
               f.tx_hash::text AS tx_hash,
               'limit'::text AS surface,
               'fill'::text AS kind,
               f.id AS ordinal,
               f.id AS row_id,
               f.block_timestamp,
               p.contract_address::text AS pair_address,
               f.maker::text AS actor,
               NULL::numeric AS offer_amount,
               NULL::numeric AS return_amount,
               NULL::numeric AS pool_return_amount,
               NULL::numeric AS book_return_amount,
               NULL::smallint AS offer_decimals,
               NULL::smallint AS ask_decimals,
               f.order_id,
               f.side::text AS side,
               f.price,
               f.token0_amount,
               f.token1_amount,
               f.commission_amount,
               f.swap_event_id,
               NULL::numeric AS asset_0_amount,
               NULL::numeric AS asset_1_amount,
               NULL::numeric AS lp_amount,
               NULL::numeric AS amount_raw,
               NULL::smallint AS fee_decimals,
               NULL::text AS token_denom,
               NULL::text AS token_contract,
               NULL::numeric AS fee_usd
        FROM limit_order_fills f
        JOIN pairs p ON p.id = f.pair_id
        WHERE ",
        );
        push_window(&mut qb, "f.block_timestamp", day_start, day_end);
    }

    if surfaces.wrap {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(
            r"
        SELECT e.block_height,
               e.tx_hash::text AS tx_hash,
               'wrap'::text AS surface,
               e.source::text AS kind,
               e.ordinal,
               e.id AS row_id,
               e.block_timestamp,
               NULL::text AS pair_address,
               NULL::text AS actor,
               NULL::numeric AS offer_amount,
               NULL::numeric AS return_amount,
               NULL::numeric AS pool_return_amount,
               NULL::numeric AS book_return_amount,
               NULL::smallint AS offer_decimals,
               NULL::smallint AS ask_decimals,
               NULL::bigint AS order_id,
               NULL::text AS side,
               NULL::numeric AS price,
               NULL::numeric AS token0_amount,
               NULL::numeric AS token1_amount,
               NULL::numeric AS commission_amount,
               NULL::bigint AS swap_event_id,
               NULL::numeric AS asset_0_amount,
               NULL::numeric AS asset_1_amount,
               NULL::numeric AS lp_amount,
               e.amount_raw,
               e.decimals AS fee_decimals,
               CASE WHEN a.is_cw20 THEN NULL ELSE a.denom::text END AS token_denom,
               a.contract_address::text AS token_contract,
               e.fee_usd
        FROM protocol_fee_events e
        JOIN assets a ON a.id = e.asset_id
        WHERE e.source IN ('wrap', 'unwrap')
          AND ",
        );
        push_window(&mut qb, "e.block_timestamp", day_start, day_end);
    }

    if first {
        return Ok(vec![]);
    }

    qb.push(") u");
    if let Some(c) = cursor {
        qb.push(" WHERE (u.block_height, u.tx_hash, u.surface, u.kind, u.ordinal, u.row_id) > (");
        qb.push_bind(c.block_height);
        qb.push(", ");
        qb.push_bind(c.tx_hash.clone());
        qb.push(", ");
        qb.push_bind(c.surface.clone());
        qb.push(", ");
        qb.push_bind(c.kind.clone());
        qb.push(", ");
        qb.push_bind(c.ordinal);
        qb.push(", ");
        qb.push_bind(c.row_id);
        qb.push(")");
    }
    qb.push(
        " ORDER BY u.block_height ASC, u.tx_hash ASC, u.surface ASC, u.kind ASC, u.ordinal ASC, u.row_id ASC LIMIT ",
    );
    qb.push_bind(limit);

    let query = qb.build_query_as::<EvidenceRawRow>();
    query.fetch_all(pool).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn evidence_sql_aliases_every_arm_and_skips_reserve_snapshots() {
        let src = include_str!("evidence_daily.rs");
        let lower = src.to_ascii_lowercase();
        let banned = ["pair_", "reserves"].concat();
        assert!(
            !lower.contains(&banned),
            "evidence SQL must not scan live reserve snapshots"
        );
        for table in [
            "swap_events",
            "liquidity_events",
            "limit_order_placements",
            "limit_order_cancellations",
            "limit_order_fills",
            "protocol_fee_events",
        ] {
            assert!(src.contains(table), "missing {table}");
        }
        let row_id_alias = ["id AS ", "row_id"].concat();
        assert_eq!(
            src.matches(&row_id_alias).count(),
            6,
            "each union arm needs an aliased row_id"
        );
        assert!(src.contains("AS offer_amount"));
        assert!(src.contains("e.source IN ('wrap', 'unwrap')"));
    }
}
