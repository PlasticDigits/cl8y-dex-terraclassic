//! #1206 anonymous UTC-day digest. Independent from the chain-linkable event tape.
use std::collections::BTreeMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use chrono::{NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{pairs::bd_plain_string, AppState};
use crate::{db::queries::evidence_digest as db, indexer::protocol_fees::FeeSource};

#[derive(Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
pub struct DigestQuery {
    pub day: String,
}
#[derive(Serialize, ToSchema)]
pub struct DigestFeeSource {
    pub source: String,
    pub event_count: i64,
    pub amount_usd: Option<String>,
    pub share_pct: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct DigestFeeToken {
    pub contract_or_denom: Option<String>,
    pub event_count: i64,
    pub amount_usd: Option<String>,
    pub share_pct: Option<String>,
    pub is_other: bool,
}
#[derive(Serialize, ToSchema)]
pub struct DigestFees {
    pub total_usd: Option<String>,
    pub by_source: Vec<DigestFeeSource>,
    pub by_token: Vec<DigestFeeToken>,
}
#[derive(Serialize, ToSchema)]
pub struct EvidenceDigest {
    pub day: String,
    pub timezone: String,
    pub complete: bool,
    /// Includes indexed gem/test markets; wrap and window counts are fee events, not principal.
    pub counts: BTreeMap<String, BTreeMap<String, i64>>,
    pub fees: DigestFees,
}
fn bad_day() -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        "day must be a nonfuture YYYY-MM-DD UTC date".into(),
    )
}
fn parse_day(raw: &str) -> Result<NaiveDate, (StatusCode, String)> {
    let day = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| bad_day())?;
    if raw.len() != 10 || day.to_string() != raw || day > Utc::now().date_naive() {
        return Err(bad_day());
    }
    Ok(day)
}
fn amount(count: i64, priced: i64, sum: Option<&BigDecimal>) -> Option<String> {
    if count == 0 {
        Some("0".into())
    } else if count == priced {
        sum.map(bd_plain_string)
    } else {
        None
    }
}
fn share(value: Option<&BigDecimal>, total: &BigDecimal) -> Option<String> {
    if *total <= BigDecimal::from(0) {
        return None;
    }
    value.map(|v| bd_plain_string(&((v * BigDecimal::from(100)) / total).with_scale(18)))
}
fn assemble(
    day: NaiveDate,
    counts: Vec<db::CountRow>,
    sources: Vec<db::SourceRow>,
    tokens: Vec<db::TokenRow>,
) -> EvidenceDigest {
    let mut activity: BTreeMap<String, BTreeMap<String, i64>> = [
        ("swap", vec!["swap"]),
        ("lp", vec!["add", "remove"]),
        ("limit", vec!["place", "cancel", "fill"]),
        ("wrap", vec!["wrap", "unwrap"]),
        ("window", vec!["ust1_mint", "ust1_redeem"]),
    ]
    .into_iter()
    .map(|(surface, kinds)| {
        (
            surface.into(),
            kinds.into_iter().map(|k| (k.into(), 0)).collect(),
        )
    })
    .collect();
    for row in counts {
        if let Some(value) = activity
            .get_mut(&row.surface)
            .and_then(|k| k.get_mut(&row.kind))
        {
            *value = row.event_count;
        }
    }
    let total_count: i64 = sources.iter().map(|r| r.event_count).sum();
    let total: BigDecimal = sources
        .iter()
        .filter_map(|r| r.priced_usd.as_ref())
        .cloned()
        .sum();
    let any_priced = sources.iter().any(|r| r.priced_count > 0);
    let by_source = FeeSource::ALL
        .into_iter()
        .map(|source| {
            let row = sources.iter().find(|r| r.source == source.as_str());
            let (count, priced, sum) = row
                .map(|r| (r.event_count, r.priced_count, r.priced_usd.as_ref()))
                .unwrap_or((0, 0, None));
            let amount_usd = amount(count, priced, sum);
            let eligible_sum = if count == 0 {
                Some(BigDecimal::from(0))
            } else if count == priced {
                sum.cloned()
            } else {
                None
            };
            let surface = if source.is_wrap_family() {
                Some("wrap")
            } else if source.is_ust1_window_family() {
                Some("window")
            } else {
                None
            };
            if let Some(surface) = surface {
                *activity
                    .get_mut(surface)
                    .unwrap()
                    .get_mut(source.as_str())
                    .unwrap() = count;
            }
            DigestFeeSource {
                source: source.as_str().into(),
                event_count: count,
                amount_usd,
                share_pct: share(eligible_sum.as_ref(), &total),
            }
        })
        .collect();
    let by_token = tokens
        .into_iter()
        .map(|r| {
            let amount_usd = amount(r.event_count, r.priced_count, r.priced_usd.as_ref());
            let share_pct = if r.event_count == r.priced_count {
                share(r.priced_usd.as_ref(), &total)
            } else {
                None
            };
            DigestFeeToken {
                contract_or_denom: r.contract_address.or(r.denom),
                event_count: r.event_count,
                amount_usd,
                share_pct,
                is_other: r.is_other,
            }
        })
        .collect();
    EvidenceDigest {
        day: day.to_string(),
        timezone: "UTC".into(),
        complete: day < Utc::now().date_naive(),
        counts: activity,
        fees: DigestFees {
            total_usd: if total_count == 0 {
                Some("0".into())
            } else if any_priced {
                Some(bd_plain_string(&total))
            } else {
                None
            },
            by_source,
            by_token,
        },
    }
}

#[utoipa::path(
    get, path = "/api/v1/evidence/digest", params(DigestQuery),
    responses((status = 200, description = "Anonymous protocol UTC-day counts and canonical fee-path mix", body = EvidenceDigest),
        (status = 400, description = "Missing, malformed, future day or unknown query parameter"),
        (status = 429, description = "Evidence admission full"),
        (status = 500, description = "Internal server error"),
        (status = 503, description = "Evidence request deadline exceeded")), tag = "Evidence"
)]
pub async fn get_evidence_digest(
    State(state): State<AppState>,
    query: Result<Query<DigestQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<EvidenceDigest>, (StatusCode, String)> {
    let Query(query) = query.map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "only one day=YYYY-MM-DD parameter is supported".into(),
        )
    })?;
    let day = parse_day(&query.day)?;
    let start = Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0).ok_or_else(bad_day)?);
    let end = start
        .checked_add_signed(chrono::Duration::days(1))
        .ok_or_else(bad_day)?;
    let sources: Vec<_> = FeeSource::ALL.iter().map(|s| s.as_str()).collect();
    let (counts, fees, tokens) = db::aggregate(&state.pool, start, end, &sources)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".into(),
            )
        })?;
    Ok(Json(assemble(day, counts, fees, tokens)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_calendar_days() {
        for bad in [
            "today",
            "2026-6-01",
            "2026-02-30",
            "2026-01-01T00:00:00Z",
            "2099-01-01",
            "2026-01-01' OR 1=1--",
        ] {
            assert!(parse_day(bad).is_err(), "{bad}");
        }
        assert!(parse_day("2026-01-01").is_ok());
    }
    #[test]
    fn canonical_idle_and_unpriced_mix_preserves_priced_headline() {
        let day = parse_day("2026-01-01").unwrap();
        let response = assemble(
            day,
            vec![db::CountRow {
                surface: "swap".into(),
                kind: "swap".into(),
                event_count: 1,
            }],
            vec![
                db::SourceRow {
                    source: "swap_amm".into(),
                    event_count: 2,
                    priced_count: 0,
                    priced_usd: None,
                },
                db::SourceRow {
                    source: "wrap".into(),
                    event_count: 1,
                    priced_count: 1,
                    priced_usd: Some(2.into()),
                },
                db::SourceRow {
                    source: "ust1_mint".into(),
                    event_count: 1,
                    priced_count: 0,
                    priced_usd: None,
                },
            ],
            vec![],
        );
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["fees"]["total_usd"], "2");
        assert_eq!(
            json["fees"]["by_source"].as_array().unwrap().len(),
            FeeSource::ALL.len()
        );
        assert!(json["fees"]["by_source"][0]["amount_usd"].is_null());
        assert_eq!(json["fees"]["by_source"][1]["amount_usd"], "0");
        assert_eq!(json["counts"]["swap"]["swap"], 1);
        assert_eq!(json["counts"]["limit"]["fill"], 0);
        assert_eq!(json["counts"]["wrap"]["wrap"], 1);
        assert_eq!(json["counts"]["window"]["ust1_mint"], 1);
        let idle = serde_json::to_value(assemble(day, vec![], vec![], vec![])).unwrap();
        assert_eq!(idle["fees"]["total_usd"], "0");
        assert!(idle["fees"]["by_source"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["share_pct"].is_null()));
    }
}
