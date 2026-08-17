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
    /// Match-quality tiers when `q` is set; falls back to 24h volume when `q` is empty.
    Relevance,
}

pub struct PairListParams<'a> {
    pub q: Option<&'a str>,
    pub asset: Option<&'a str>,
    pub sort: PairListSort,
    pub sort_desc: bool,
    pub limit: i64,
    pub offset: i64,
}

/// Escape SQL `LIKE`/`ILIKE` wildcard metacharacters (`\`, `%`, `_`) in a user-supplied
/// search term so they match literally instead of as wildcards (GitLab #459 / SEC-I04 F02).
///
/// Without this, `?q=%` produces an `ILIKE '%...%'` pattern that matches every pair (and forces
/// a full sequential scan), and `?q=_` matches any single character. PostgreSQL's default
/// `LIKE`/`ILIKE` escape character is the backslash, so escaping these three characters in the
/// bound pattern value is sufficient — no explicit `ESCAPE` clause is required. Backslash is
/// escaped first to avoid double-escaping the escapes we add.
fn escape_like_pattern(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Split `XXX YYY` or `XXX/YYY` pair-symbol queries into two lowercase tokens.
fn parse_pair_symbol_tokens(q: &str) -> Option<(String, String)> {
    let trimmed = q.trim();
    let parts: Vec<&str> = if trimmed.contains('/') {
        trimmed.split('/').collect()
    } else {
        trimmed.split_whitespace().collect()
    };
    if parts.len() != 2 {
        return None;
    }
    let t0 = parts[0].trim().to_ascii_lowercase();
    let t1 = parts[1].trim().to_ascii_lowercase();
    if t0.is_empty() || t1.is_empty() {
        return None;
    }
    Some((t0, t1))
}

fn push_asset_leg_exact_match(qb: &mut QueryBuilder<'_, Postgres>, alias: &str, token: String) {
    qb.push("(LOWER(");
    qb.push(alias);
    qb.push(".symbol) = ");
    qb.push_bind(token.clone());
    qb.push(" OR LOWER(");
    qb.push(alias);
    qb.push(".name) = ");
    qb.push_bind(token);
    qb.push(")");
}

fn push_pair_symbol_pair_exact_match(
    qb: &mut QueryBuilder<'_, Postgres>,
    t0: String,
    t1: String,
) {
    qb.push("((");
    push_asset_leg_exact_match(qb, "a0", t0.clone());
    qb.push(" AND ");
    push_asset_leg_exact_match(qb, "a1", t1.clone());
    qb.push(") OR (");
    push_asset_leg_exact_match(qb, "a0", t1);
    qb.push(" AND ");
    push_asset_leg_exact_match(qb, "a1", t0);
    qb.push("))");
}

fn push_pair_relevance_score(qb: &mut QueryBuilder<'_, Postgres>, q: &str) {
    let trimmed = q.trim();
    let q_lower = trimmed.to_ascii_lowercase();
    let pattern = format!("%{}%", escape_like_pattern(trimmed));

    qb.push(" GREATEST(");
    // Tier 5: exact pair address or exact two-token pair symbol/name match.
    qb.push("(CASE WHEN LOWER(p.contract_address) = ");
    qb.push_bind(q_lower.clone());
    qb.push(" THEN 5 ELSE 0 END)");
    if let Some((t0, t1)) = parse_pair_symbol_tokens(trimmed) {
        qb.push(", (CASE WHEN ");
        push_pair_symbol_pair_exact_match(&mut *qb, t0, t1);
        qb.push(" THEN 5 ELSE 0 END)");
    }
    // Tier 4: exact token contract or native denom.
    qb.push(", (CASE WHEN LOWER(COALESCE(a0.contract_address, '')) = ");
    qb.push_bind(q_lower.clone());
    qb.push(" OR LOWER(COALESCE(a1.contract_address, '')) = ");
    qb.push_bind(q_lower.clone());
    qb.push(" OR LOWER(COALESCE(a0.denom, '')) = ");
    qb.push_bind(q_lower.clone());
    qb.push(" OR LOWER(COALESCE(a1.denom, '')) = ");
    qb.push_bind(q_lower.clone());
    qb.push(" THEN 4 ELSE 0 END)");
    // Tier 3: exact token symbol on either leg.
    qb.push(", (CASE WHEN LOWER(a0.symbol) = ");
    qb.push_bind(q_lower.clone());
    qb.push(" OR LOWER(a1.symbol) = ");
    qb.push_bind(q_lower.clone());
    qb.push(" THEN 3 ELSE 0 END)");
    // Tier 2: token name substring on either leg.
    qb.push(", (CASE WHEN a0.name ILIKE ");
    qb.push_bind(pattern.clone());
    qb.push(" OR a1.name ILIKE ");
    qb.push_bind(pattern.clone());
    qb.push(" THEN 2 ELSE 0 END)");
    // Tier 1: general substring fallback (also covers partial symbol/address matches).
    qb.push(", (CASE WHEN p.contract_address ILIKE ");
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
    qb.push(" THEN 1 ELSE 0 END)");
    qb.push(")");
}

fn push_asset_leg_ilike_match(qb: &mut QueryBuilder<'_, Postgres>, alias: &str, pattern: String) {
    qb.push("(LOWER(");
    qb.push(alias);
    qb.push(".symbol) ILIKE ");
    qb.push_bind(pattern.clone());
    qb.push(" OR LOWER(");
    qb.push(alias);
    qb.push(".name) ILIKE ");
    qb.push_bind(pattern.clone());
    qb.push(" OR LOWER(COALESCE(");
    qb.push(alias);
    qb.push(".contract_address, '')) ILIKE ");
    qb.push_bind(pattern.clone());
    qb.push(" OR LOWER(COALESCE(");
    qb.push(alias);
    qb.push(".denom, '')) ILIKE ");
    qb.push_bind(pattern);
    qb.push(")");
}

fn push_pair_list_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    q: Option<&str>,
    asset: Option<&str>,
) {
    if let Some(q) = q.filter(|s| !s.trim().is_empty()) {
        let trimmed = q.trim();
        let pattern = format!("%{}%", escape_like_pattern(&trimmed.to_ascii_lowercase()));
        qb.push(" AND (p.contract_address ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR ");
        push_asset_leg_ilike_match(qb, "a0", pattern.clone());
        qb.push(" OR ");
        push_asset_leg_ilike_match(qb, "a1", pattern.clone());
        if let Some((t0, t1)) = parse_pair_symbol_tokens(trimmed) {
            let p0 = format!("%{}%", escape_like_pattern(&t0));
            let p1 = format!("%{}%", escape_like_pattern(&t1));
            qb.push(" OR ((");
            push_asset_leg_ilike_match(qb, "a0", p0.clone());
            qb.push(" AND ");
            push_asset_leg_ilike_match(qb, "a1", p1.clone());
            qb.push(") OR (");
            push_asset_leg_ilike_match(qb, "a0", p1);
            qb.push(" AND ");
            push_asset_leg_ilike_match(qb, "a1", p0);
            qb.push("))");
        }
        qb.push(")");
    }

    if let Some(asset) = asset.filter(|s| !s.trim().is_empty()) {
        let a = asset.trim().to_string();
        qb.push(" AND (a0.contract_address = ");
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

fn push_pair_list_order_by(
    qb: &mut QueryBuilder<'_, Postgres>,
    sort: PairListSort,
    sort_desc: bool,
    q: Option<&str>,
) {
    qb.push(" ORDER BY ");
    let desc = if sort_desc { " DESC" } else { " ASC" };
    match sort {
        PairListSort::Relevance => {
            if let Some(q) = q.filter(|s| !s.trim().is_empty()) {
                push_pair_relevance_score(qb, q);
                qb.push(desc);
                qb.push(", COALESCE(pv.volume_quote, 0) DESC, p.id ASC");
            } else {
                qb.push("COALESCE(pv.volume_quote, 0) DESC, p.id ASC");
            }
        }
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
            qb.push("COALESCE(pv.volume_quote, 0)");
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
                p.created_at_block, p.created_at, p.updated_at, pv.volume_quote AS volume_quote_24h
         FROM pairs p
         INNER JOIN assets a0 ON a0.id = p.asset_0_id
         INNER JOIN assets a1 ON a1.id = p.asset_1_id
         LEFT JOIN pair_volume_24h pv ON pv.pair_id = p.id
         WHERE 1=1",
    );
    push_pair_list_filters(&mut qb, params.q, params.asset);
    push_pair_list_order_by(&mut qb, params.sort, params.sort_desc, params.q);
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

pub async fn count_pairs(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pairs")
        .fetch_one(pool)
        .await
}

pub async fn count_pairs_created_since(
    pool: &PgPool,
    since: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pairs WHERE created_at >= $1")
        .bind(since)
        .fetch_one(pool)
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

#[cfg(test)]
mod escape_like_tests {
    use super::escape_like_pattern;

    // GitLab #459 (SEC-I04 F02): wildcard metacharacters in the search term are neutralized
    // so `?q=%` cannot match every row and `?q=_` cannot match any single character.
    #[test]
    fn escapes_percent_underscore_backslash() {
        assert_eq!(escape_like_pattern("%"), "\\%");
        assert_eq!(escape_like_pattern("_"), "\\_");
        assert_eq!(escape_like_pattern("\\"), "\\\\");
    }

    #[test]
    fn backslash_escaped_before_wildcards_no_double_escape() {
        // A literal `\%` must become `\\\%` (escaped backslash + escaped percent), not `\\%`.
        assert_eq!(escape_like_pattern("\\%"), "\\\\\\%");
    }

    #[test]
    fn ordinary_text_unchanged() {
        assert_eq!(escape_like_pattern("EMBER"), "EMBER");
        assert_eq!(escape_like_pattern("terra1abc"), "terra1abc");
    }

    #[test]
    fn mixed_term_escaped_in_place() {
        assert_eq!(escape_like_pattern("a%b_c"), "a\\%b\\_c");
    }
}
