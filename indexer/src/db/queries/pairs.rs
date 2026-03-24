use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, FromRow)]
pub struct PairRow {
    pub id: i32,
    pub contract_address: String,
    pub asset_0_id: i32,
    pub asset_1_id: i32,
    pub lp_token: Option<String>,
    pub fee_bps: Option<i16>,
    pub hooks: Vec<String>,
    pub created_at_block: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One row from the paginated pair list (includes 24h quote volume from swap_events).
#[derive(Debug, Clone, FromRow)]
pub struct PairListRow {
    #[sqlx(flatten)]
    pub pair: PairRow,
    pub volume_quote_24h: Option<BigDecimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PairListSort {
    #[default]
    Id,
    Fee,
    Created,
    Symbol,
    Volume24h,
}

pub struct PairListParams<'a> {
    pub q: Option<&'a str>,
    pub asset: Option<&'a str>,
    pub sort: PairListSort,
    pub sort_desc: bool,
    pub limit: i64,
    pub offset: i64,
}

fn push_pair_list_filters(qb: &mut QueryBuilder<'_, Postgres>, q: Option<&str>, asset: Option<&str>) {
    if let Some(q) = q.filter(|s| !s.trim().is_empty()) {
        let pattern = format!("%{}%", q.trim());
        qb.push(
            " AND (p.contract_address ILIKE ",
        );
        qb.push_bind(pattern.clone());
        qb.push(" OR a0.symbol ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR a1.symbol ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR COALESCE(a0.contract_address, '') ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR COALESCE(a1.contract_address, '') ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR COALESCE(a0.denom, '') ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR COALESCE(a1.denom, '') ILIKE ");
        qb.push_bind(pattern);
        qb.push(")");
    }

    if let Some(asset) = asset.filter(|s| !s.trim().is_empty()) {
        let a = asset.trim().to_string();
        qb.push(
            " AND (a0.contract_address = ",
        );
        qb.push_bind(a.clone());
        qb.push(" OR a1.contract_address = ");
        qb.push_bind(a.clone());
        qb.push(" OR a0.denom = ");
        qb.push_bind(a.clone());
        qb.push(" OR a1.denom = ");
        qb.push_bind(a);
        qb.push(")");
    }
}

fn push_pair_list_order_by(qb: &mut QueryBuilder<'_, Postgres>, sort: PairListSort, sort_desc: bool) {
    qb.push(" ORDER BY ");
    let desc = if sort_desc { " DESC" } else { " ASC" };
    match sort {
        PairListSort::Id => {
            qb.push("p.id");
            qb.push(desc);
        }
        PairListSort::Fee => {
            qb.push("p.fee_bps");
            qb.push(desc);
            qb.push(" NULLS LAST, p.id ASC");
        }
        PairListSort::Created => {
            qb.push("p.created_at_block");
            qb.push(desc);
            qb.push(" NULLS LAST, p.id ASC");
        }
        PairListSort::Symbol => {
            qb.push("(LOWER(a0.symbol) || '/' || LOWER(a1.symbol))");
            qb.push(desc);
            qb.push(", p.id ASC");
        }
        PairListSort::Volume24h => {
            qb.push("COALESCE(se.volume_quote_24h, 0)");
            qb.push(desc);
            qb.push(", p.id ASC");
        }
    }
}

pub async fn count_pairs_filtered(
    pool: &PgPool,
    q: Option<&str>,
    asset: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT COUNT(*)::bigint FROM pairs p
         INNER JOIN assets a0 ON a0.id = p.asset_0_id
         INNER JOIN assets a1 ON a1.id = p.asset_1_id
         WHERE 1=1",
    );
    push_pair_list_filters(&mut qb, q, asset);
    let total: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(total)
}

pub async fn list_pairs_filtered(
    pool: &PgPool,
    params: PairListParams<'_>,
) -> Result<Vec<PairListRow>, sqlx::Error> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT p.id, p.contract_address, p.asset_0_id, p.asset_1_id, p.lp_token, p.fee_bps, p.hooks,
                p.created_at_block, p.created_at, p.updated_at, se.volume_quote_24h
         FROM pairs p
         INNER JOIN assets a0 ON a0.id = p.asset_0_id
         INNER JOIN assets a1 ON a1.id = p.asset_1_id
         LEFT JOIN (
           SELECT pair_id, SUM(return_amount) AS volume_quote_24h
           FROM swap_events
           WHERE block_timestamp >= NOW() - INTERVAL '24 hours'
           GROUP BY pair_id
         ) se ON se.pair_id = p.id
         WHERE 1=1",
    );
    push_pair_list_filters(&mut qb, params.q, params.asset);
    push_pair_list_order_by(&mut qb, params.sort, params.sort_desc);
    qb.push(" LIMIT ");
    qb.push_bind(params.limit);
    qb.push(" OFFSET ");
    qb.push_bind(params.offset);

    qb.build_query_as::<PairListRow>().fetch_all(pool).await
}

pub async fn upsert_pair(
    pool: &PgPool,
    contract_address: &str,
    asset_0_id: i32,
    asset_1_id: i32,
    lp_token: Option<&str>,
    fee_bps: Option<i16>,
    hooks: &[String],
    created_at_block: Option<i64>,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps, hooks, created_at_block)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (contract_address)
           DO UPDATE SET lp_token = COALESCE(EXCLUDED.lp_token, pairs.lp_token),
                        fee_bps = COALESCE(EXCLUDED.fee_bps, pairs.fee_bps),
                        hooks = EXCLUDED.hooks,
                        updated_at = NOW()
         RETURNING id",
    )
    .bind(contract_address)
    .bind(asset_0_id)
    .bind(asset_1_id)
    .bind(lp_token)
    .bind(fee_bps)
    .bind(hooks)
    .bind(created_at_block)
    .fetch_one(pool)
    .await
}

pub async fn get_pair_by_address(
    pool: &PgPool,
    contract_address: &str,
) -> Result<Option<PairRow>, sqlx::Error> {
    sqlx::query_as::<_, PairRow>("SELECT * FROM pairs WHERE contract_address = $1")
        .bind(contract_address)
        .fetch_optional(pool)
        .await
}

pub async fn get_all_pairs(pool: &PgPool) -> Result<Vec<PairRow>, sqlx::Error> {
    sqlx::query_as::<_, PairRow>("SELECT * FROM pairs ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn get_pairs_for_asset(
    pool: &PgPool,
    asset_id: i32,
) -> Result<Vec<PairRow>, sqlx::Error> {
    sqlx::query_as::<_, PairRow>(
        "SELECT * FROM pairs WHERE asset_0_id = $1 OR asset_1_id = $1 ORDER BY id",
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
}

pub async fn update_pair_config(
    pool: &PgPool,
    pair_id: i32,
    fee_bps: Option<i16>,
    hooks: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE pairs SET fee_bps = COALESCE($2, fee_bps), hooks = $3, updated_at = NOW()
         WHERE id = $1",
    )
    .bind(pair_id)
    .bind(fee_bps)
    .bind(hooks)
    .execute(pool)
    .await?;
    Ok(())
}
