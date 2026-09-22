//! UTC-day redacted evidence rows for `GET /api/v1/evidence/daily` (GitLab #1205).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceSortKey {
    pub block_height: i64,
    pub tx_hash: String,
    pub surface: String,
    pub kind: String,
    pub ordinal: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct EvidenceRawRow {
    pub block_height: i64,
    pub tx_hash: String,
    pub surface: String,
    pub kind: String,
    pub ordinal: i64,
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

    let swap_select = r"
        SELECT s.block_height, s.tx_hash, 'swap'::text AS surface, 'swap'::text AS kind,
               COALESCE(s.swap_index, 0)::bigint AS ordinal,
               s.block_timestamp, p.contract_address AS pair_address,
               s.sender AS actor,
               s.offer_amount, s.return_amount, s.pool_return_amount, s.book_return_amount,
               a0.decimals AS offer_decimals, a1.decimals AS ask_decimals,
               NULL::bigint AS order_id, NULL::text AS side, NULL::numeric AS price,
               NULL::numeric AS token0_amount, NULL::numeric AS token1_amount,
               NULL::numeric AS commission_amount, NULL::bigint AS swap_event_id,
               NULL::numeric AS asset_0_amount, NULL::numeric AS asset_1_amount, NULL::numeric AS lp_amount,
               NULL::numeric AS amount_raw, NULL::smallint AS fee_decimals,
               NULL::text AS token_denom, NULL::text AS token_contract, NULL::numeric AS fee_usd
        FROM swap_events s
        JOIN pairs p ON p.id = s.pair_id
        JOIN assets a0 ON a0.id = s.offer_asset_id
        JOIN assets a1 ON a1.id = s.ask_asset_id
        WHERE s.block_timestamp >= ";
    let day_end_sql = " AND s.block_timestamp < ";

    if surfaces.swap {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(swap_select);
        qb.push_bind(day_start);
        qb.push(day_end_sql);
        qb.push_bind(day_end);
    }

    if surfaces.lp {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(
            r"
        SELECT le.block_height, le.tx_hash, 'lp'::text AS surface, le.event_type AS kind,
               le.id AS ordinal,
               le.block_timestamp, p.contract_address AS pair_address,
               le.provider AS actor,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::smallint, NULL::smallint,
               NULL::bigint, NULL::text, NULL::numeric,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::bigint,
               le.asset_0_amount, le.asset_1_amount, le.lp_amount,
               NULL::numeric, NULL::smallint, NULL::text, NULL::text, NULL::numeric
        FROM liquidity_events le
        JOIN pairs p ON p.id = le.pair_id
        WHERE le.block_timestamp >= ",
        );
        qb.push_bind(day_start);
        qb.push(" AND le.block_timestamp < ");
        qb.push_bind(day_end);
    }

    if surfaces.limit {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(
            r"
        SELECT pl.block_height, pl.tx_hash, 'limit'::text AS surface, 'place'::text AS kind,
               pl.id AS ordinal,
               pl.block_timestamp, p.contract_address AS pair_address,
               pl.owner AS actor,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::smallint, NULL::smallint,
               pl.order_id, pl.side, pl.price,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::bigint,
               NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::numeric, NULL::smallint, NULL::text, NULL::text, NULL::numeric
        FROM limit_order_placements pl
        JOIN pairs p ON p.id = pl.pair_id
        WHERE pl.block_timestamp >= ",
        );
        qb.push_bind(day_start);
        qb.push(" AND pl.block_timestamp < ");
        qb.push_bind(day_end);

        qb.push(" UNION ALL ");
        qb.push(
            r"
        SELECT c.block_height, c.tx_hash, 'limit'::text AS surface, 'cancel'::text AS kind,
               c.id AS ordinal,
               c.block_timestamp, p.contract_address AS pair_address,
               c.owner AS actor,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::smallint, NULL::smallint,
               c.order_id, NULL::text, NULL::numeric,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::bigint,
               NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::numeric, NULL::smallint, NULL::text, NULL::text, NULL::numeric
        FROM limit_order_cancellations c
        JOIN pairs p ON p.id = c.pair_id
        WHERE c.block_timestamp >= ",
        );
        qb.push_bind(day_start);
        qb.push(" AND c.block_timestamp < ");
        qb.push_bind(day_end);

        qb.push(" UNION ALL ");
        qb.push(
            r"
        SELECT f.block_height, f.tx_hash, 'limit'::text AS surface, 'fill'::text AS kind,
               f.id AS ordinal,
               f.block_timestamp, p.contract_address AS pair_address,
               f.maker AS actor,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::smallint, NULL::smallint,
               f.order_id, f.side, f.price,
               f.token0_amount, f.token1_amount, f.commission_amount, f.swap_event_id,
               NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::numeric, NULL::smallint, NULL::text, NULL::text, NULL::numeric
        FROM limit_order_fills f
        JOIN pairs p ON p.id = f.pair_id
        WHERE f.block_timestamp >= ",
        );
        qb.push_bind(day_start);
        qb.push(" AND f.block_timestamp < ");
        qb.push_bind(day_end);
    }

    if surfaces.wrap {
        if !first {
            qb.push(" UNION ALL ");
        }
        first = false;
        qb.push(
            r"
        SELECT e.block_height, e.tx_hash, 'wrap'::text AS surface, e.source AS kind,
               e.ordinal,
               e.block_timestamp, NULL::text AS pair_address,
               NULL::text AS actor,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::numeric,
               NULL::smallint, NULL::smallint,
               NULL::bigint, NULL::text, NULL::numeric,
               NULL::numeric, NULL::numeric, NULL::numeric, NULL::bigint,
               NULL::numeric, NULL::numeric, NULL::numeric,
               e.amount_raw, e.decimals AS fee_decimals,
               CASE WHEN a.is_cw20 THEN NULL ELSE a.denom END AS token_denom,
               a.contract_address AS token_contract,
               e.fee_usd
        FROM protocol_fee_events e
        JOIN assets a ON a.id = e.asset_id
        WHERE e.source IN ('wrap', 'unwrap')
          AND e.block_timestamp >= ",
        );
        qb.push_bind(day_start);
        qb.push(" AND e.block_timestamp < ");
        qb.push_bind(day_end);
    }

    if first {
        return Ok(vec![]);
    }

    qb.push(") u");
    if let Some(c) = cursor {
        qb.push(" WHERE (u.block_height, u.tx_hash, u.surface, u.kind, u.ordinal) > (");
        qb.push_bind(c.block_height);
        qb.push(", ");
        qb.push_bind(c.tx_hash.clone());
        qb.push(", ");
        qb.push_bind(c.surface.clone());
        qb.push(", ");
        qb.push_bind(c.kind.clone());
        qb.push(", ");
        qb.push_bind(c.ordinal);
        qb.push(")");
    }
    qb.push(
        " ORDER BY u.block_height ASC, u.tx_hash ASC, u.surface ASC, u.kind ASC, u.ordinal ASC LIMIT ",
    );
    qb.push_bind(limit);

    let query = qb.build_query_as::<EvidenceRawRow>();
    query.fetch_all(pool).await
}
