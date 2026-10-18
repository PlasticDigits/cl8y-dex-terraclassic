//! Rolling admission is separate from poller leadership and cheap liveness.
//! Explicit bounded scenarios consume the unchanged local quote engine only.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use bech32::FromBase32;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{sync::Semaphore, time::Instant};
use utoipa::ToSchema;

use super::{local_quote, route_solver::SolveRouteParams, AppState};

pub const SCENARIOS_ENV: &str = "API_READY_SCENARIOS";
const MAX_PROFILE_BYTES: usize = 2_048;
const HANDLER_BUDGET: Duration = Duration::from_millis(500);
const DATABASE_BUDGET: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessReason {
    ConfigurationMissing,
    ConfigurationInvalid,
    Busy,
    DatabaseUnavailable,
    QuotesUnavailable,
    Deadline,
}

#[derive(Clone)]
pub struct ApiReadinessProfile {
    scenarios: Result<Vec<Scenario>, ReadinessReason>,
    gate: Arc<Semaphore>,
    startup_admitted: Arc<AtomicBool>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    token_in: String,
    token_out: String,
    amount_in: String,
    #[serde(default = "default_spread")]
    max_spread: String,
}
fn default_spread() -> String {
    "0.01".into()
}
fn native(address: &str) -> bool {
    matches!(address, "uluna" | "uusd")
}
fn cw20(address: &str) -> bool {
    if address.len() > 90 || address.to_ascii_lowercase() != address {
        return false;
    }
    bech32::decode(address).is_ok_and(|(hrp, data, variant)| {
        hrp == "terra"
            && variant == bech32::Variant::Bech32
            && Vec::<u8>::from_base32(&data).is_ok_and(|bytes| matches!(bytes.len(), 20 | 32))
    })
}
fn positive_raw(value: &str) -> Option<u128> {
    (!value.is_empty()
        && value.len() <= 39
        && !value.starts_with('0')
        && value.bytes().all(|byte| byte.is_ascii_digit()))
    .then(|| value.parse::<u128>().ok())
    .flatten()
    .filter(|amount| *amount > 0)
}
fn valid_spread(value: &str) -> bool {
    let Some(fraction) = value.strip_prefix("0.") else {
        return false;
    };
    (1..=6).contains(&fraction.len())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.parse::<u32>().is_ok_and(|raw| {
            let millionths = raw * 10u32.pow(6 - fraction.len() as u32);
            (1..=300_000).contains(&millionths)
        })
}
impl ApiReadinessProfile {
    pub fn from_env() -> Self {
        match std::env::var(SCENARIOS_ENV) {
            Ok(raw) => Self::from_json(&raw),
            Err(std::env::VarError::NotPresent) => Self::missing(),
            Err(_) => Self::invalid(),
        }
    }
    pub fn from_json(raw: &str) -> Self {
        let parsed = (raw.len() <= MAX_PROFILE_BYTES)
            .then(|| serde_json::from_str::<Vec<Scenario>>(raw).ok())
            .flatten()
            .filter(|rows| (1..=2).contains(&rows.len()))
            .filter(|rows| {
                rows.iter().enumerate().all(|(index, scenario)| {
                    scenario.token_in != scenario.token_out
                        && (native(&scenario.token_in) || native(&scenario.token_out))
                        && [&scenario.token_in, &scenario.token_out]
                            .into_iter()
                            .all(|address| native(address) || cw20(address))
                        && positive_raw(&scenario.amount_in).is_some()
                        && valid_spread(&scenario.max_spread)
                        && !rows[..index].iter().any(|prior| {
                            prior.token_in == scenario.token_in
                                && prior.token_out == scenario.token_out
                                && prior.amount_in == scenario.amount_in
                        })
                })
            });
        Self {
            scenarios: parsed.ok_or(ReadinessReason::ConfigurationInvalid),
            gate: Arc::new(Semaphore::new(1)),
            startup_admitted: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn missing() -> Self {
        Self {
            scenarios: Err(ReadinessReason::ConfigurationMissing),
            gate: Arc::new(Semaphore::new(1)),
            startup_admitted: Arc::new(AtomicBool::new(false)),
        }
    }
    fn invalid() -> Self {
        Self {
            scenarios: Err(ReadinessReason::ConfigurationInvalid),
            gate: Arc::new(Semaphore::new(1)),
            startup_admitted: Arc::new(AtomicBool::new(false)),
        }
    }
}
impl Scenario {
    fn params(&self) -> SolveRouteParams {
        SolveRouteParams {
            token_in: self.token_in.clone(),
            token_out: self.token_out.clone(),
            amount_in: Some(self.amount_in.clone()),
            amount_out: None,
            max_spread: Some(self.max_spread.clone()),
            protection_policy: Some("quote_relative_v1".into()),
            hybrid_optimize: Some(true),
            pool_only: None,
            max_maker_fills: Some(8),
            trader: None,
            sender: None,
            discount_bps: None,
        }
    }
}

/// Only fixed reason classes and a bounded count; no configured financial identities,
/// amounts, actors, quote bodies, database errors or provider data leave this endpoint.
#[derive(Serialize, ToSchema)]
pub struct ApiReadinessResponse {
    pub ready: bool,
    pub reason: Option<ReadinessReason>,
    pub scenario_count: u8,
}

fn executable(body: &Value, scenario: &Scenario, now: i64) -> bool {
    let metadata = &body["execution_metadata"];
    let output = body["estimated_amount_out_net"]
        .as_str()
        .and_then(positive_raw);
    let floor = metadata["minimum_receive"].as_str().and_then(positive_raw);
    body["token_in"] == scenario.token_in
        && body["token_out"] == scenario.token_out
        && metadata["input_amount"] == scenario.amount_in
        && matches!(
            body["quote_kind"].as_str(),
            Some("indexer_pool_db" | "indexer_hybrid_db")
        )
        && body["execution_supported"].as_bool() != Some(false)
        && metadata["execution_supported"].as_bool() != Some(false)
        && output
            .zip(floor)
            .is_some_and(|(output, floor)| floor <= output)
        && body["snapshot_age_ms"]
            .as_i64()
            .is_some_and(|age| (0..=20_000).contains(&age))
        && metadata["observed_at_ms"]
            .as_i64()
            .zip(metadata["expires_at_ms"].as_i64())
            .is_some_and(|(observed, expires)| {
                observed <= now
                    && observed > 0
                    && expires > now
                    && expires > observed
                    && expires.saturating_sub(observed)
                        <= local_quote::LOCAL_QUOTE_SIGNING_WINDOW_MS
            })
        && body["router_operations"]
            .as_array()
            .zip(metadata["hops"].as_array())
            .is_some_and(|(operations, hops)| {
                (1..=4).contains(&hops.len())
                    && operations.len() == hops.len()
                    && operations.iter().zip(hops).all(|(operation, hop)| {
                        let floor = hop["min_return"].as_str().and_then(positive_raw);
                        floor.is_some()
                            && operation["terra_swap"]["min_return"]
                                .as_str()
                                .and_then(positive_raw)
                                == floor
                    })
            })
}

async fn evaluate(
    state: &AppState,
    scenarios: &[Scenario],
    deadline: Instant,
) -> Result<(), ReadinessReason> {
    // O(1) database readiness only. No chain/LCD client is called by this route.
    // Both reviewed schema67 and current68 are valid serving capabilities.
    match tokio::time::timeout_at(
        deadline.min(Instant::now() + DATABASE_BUDGET),
        crate::database_security::cohorts_ready(&state.pool),
    )
    .await
    {
        Ok(Ok(_)) if Instant::now() < deadline => {}
        _ => return Err(ReadinessReason::DatabaseUnavailable),
    }
    for scenario in scenarios {
        let params = scenario.params();
        let Json(body) = local_quote::solve(state, &params)
            .await
            .map_err(|_| ReadinessReason::QuotesUnavailable)?;
        let progress = local_quote::progress(state, &params)
            .map_err(|_| ReadinessReason::QuotesUnavailable)?;
        if Instant::now() >= deadline {
            return Err(ReadinessReason::Deadline);
        }
        // Reuse the engine's current cache eligibility check so a known negative,
        // changed generation or stale original observation cannot authorize cutover.
        if progress.quote_eligibility.as_deref() != Some("executable_quote")
            || progress.quote_blocker.is_some()
            || progress.quote_generation.as_deref()
                != body["execution_metadata"]["generation"].as_str()
            || !executable(&body, scenario, chrono::Utc::now().timestamp_millis())
        {
            return Err(ReadinessReason::QuotesUnavailable);
        }
    }
    Ok(())
}

async fn check(state: &AppState, profile: &ApiReadinessProfile) -> Result<(), ReadinessReason> {
    match &profile.scenarios {
        Err(reason) => Err(*reason),
        Ok(scenarios) => match profile.gate.clone().try_acquire_owned() {
            Err(_) => Err(ReadinessReason::Busy),
            Ok(_permit) => {
                let deadline = Instant::now() + HANDLER_BUDGET;
                match tokio::time::timeout_at(deadline, evaluate(state, scenarios, deadline)).await
                {
                    Ok(result) if Instant::now() < deadline => result,
                    _ => Err(ReadinessReason::Deadline),
                }
            }
        },
    }
}
fn no_store(body: impl Serialize, passed: bool) -> Response {
    let mut response = (
        if passed {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(body),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[utoipa::path(get, path = "/ready-api", responses(
    (status = 200, description = "Configured native quote scenarios are executable from original local observations; database serving capability is ready. Poller leadership is independent.", body = ApiReadinessResponse),
    (status = 503, description = "Explicit scenarios, database or cached executable quotes are unavailable. No request starts provider observation.", body = ApiReadinessResponse)
), tag = "Health")]
pub async fn ready_api(
    State(state): State<AppState>,
    Extension(profile): Extension<ApiReadinessProfile>,
) -> Response {
    let outcome = check(&state, &profile).await;
    let response = ApiReadinessResponse {
        ready: outcome.is_ok(),
        reason: outcome.err(),
        scenario_count: profile
            .scenarios
            .as_ref()
            .map_or(0, |rows| rows.len() as u8),
    };
    let passed = response.ready;
    no_store(response, passed)
}

/// Process-owned startup admission only. Never stores or authorizes a quote.
/// Memory resets on process restart; no writable-layer/volume sentinel can survive.
#[derive(Serialize, ToSchema)]
pub struct ApiServingHealthResponse {
    pub admitted: bool,
    pub reason: Option<ReadinessReason>,
    pub scenario_count: u8,
}
#[utoipa::path(get, path = "/health/serving", responses(
    (status = 200, description = "This process passed initial full API-usefulness admission. Thereafter cheap liveness preserves proxy routing; current quote readiness is reported separately.", body = ApiServingHealthResponse),
    (status = 503, description = "Initial process-owned API-usefulness admission has not passed. The old serving instance must remain.", body = ApiServingHealthResponse)
), tag = "Health")]
pub async fn serving_health(
    State(state): State<AppState>,
    Extension(profile): Extension<ApiReadinessProfile>,
) -> Response {
    let outcome = if profile.startup_admitted.load(Ordering::Acquire) {
        Ok(())
    } else {
        let result = check(&state, &profile).await;
        if result.is_ok() {
            profile.startup_admitted.store(true, Ordering::Release);
        }
        result
    };
    let response = ApiServingHealthResponse {
        admitted: outcome.is_ok(),
        reason: outcome.err(),
        scenario_count: profile
            .scenarios
            .as_ref()
            .map_or(0, |rows| rows.len() as u8),
    };
    let passed = response.admitted;
    no_store(response, passed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bech32::ToBase32;
    use serde_json::json;
    fn address(seed: u8) -> String {
        bech32::encode("terra", vec![seed; 32].to_base32(), bech32::Variant::Bech32).unwrap()
    }
    #[test]
    fn explicit_native_profiles_are_bounded_and_reject_ambiguous_or_actor_configuration() {
        let token = address(1);
        let valid = json!([{"token_in":"uusd","token_out":token,"amount_in":"1000000"}]);
        assert!(ApiReadinessProfile::from_json(&valid.to_string())
            .scenarios
            .is_ok());
        assert_eq!(
            ApiReadinessProfile::missing().scenarios.err(),
            Some(ReadinessReason::ConfigurationMissing)
        );
        let mut rejected = vec![
            json!([]),
            json!({}),
            json!([valid[0].clone(), valid[0].clone()]),
            json!([valid[0].clone(), valid[0].clone(), valid[0].clone()]),
        ];
        for (key, value) in [
            ("token_in", json!(address(2))),
            ("token_out", json!("terra1badchecksum")),
            ("token_out", json!("uusd")),
            ("amount_in", json!("0")),
            ("amount_in", json!("01")),
            ("amount_in", json!("1e6")),
            (
                "amount_in",
                json!("340282366920938463463374607431768211456"),
            ),
            ("max_spread", json!("0.31")),
            ("max_spread", json!("0.0")),
            ("max_spread", json!("1e999999999")),
            ("trader", json!(address(3))),
        ] {
            let mut profile = valid.clone();
            profile[0][key] = value;
            rejected.push(profile);
        }
        for value in rejected {
            assert_eq!(
                ApiReadinessProfile::from_json(&value.to_string())
                    .scenarios
                    .err(),
                Some(ReadinessReason::ConfigurationInvalid)
            );
        }
        assert_eq!(
            ApiReadinessProfile::from_json(&" ".repeat(MAX_PROFILE_BYTES + 1))
                .scenarios
                .err(),
            Some(ReadinessReason::ConfigurationInvalid)
        );
    }
    #[test]
    fn execution_contract_rejects_nonexecutables_and_preserves_original_signing_window() {
        let now = 2_000_000;
        let scenario = Scenario {
            token_in: "uusd".into(),
            token_out: address(1),
            amount_in: "1000000".into(),
            max_spread: default_spread(),
        };
        let body = json!({"token_in":"uusd","token_out":scenario.token_out,"quote_kind":"indexer_pool_db","estimated_amount_out_net":"90","snapshot_age_ms":1000,
            "router_operations":[{"terra_swap":{"min_return":"80"}}],"execution_metadata":{"input_amount":"1000000","minimum_receive":"80","observed_at_ms":now-21000,"expires_at_ms":now-21000+local_quote::LOCAL_QUOTE_SIGNING_WINDOW_MS,"hops":[{"min_return":"80"}]}});
        // Reviewed signing lasts30min; producer freshness is independently checked
        // by the engine's current cache eligibility, not a new20s signing expiry.
        assert!(executable(&body, &scenario, now));
        for (path, value) in [
            ("/quote_kind", json!("indexer_route_only")),
            ("/execution_supported", json!(false)),
            ("/execution_metadata/execution_supported", json!(false)),
            ("/execution_metadata/minimum_receive", json!("0")),
            ("/execution_metadata/input_amount", json!("2")),
            ("/execution_metadata/observed_at_ms", json!(now + 1)),
            ("/execution_metadata/expires_at_ms", json!(now)),
            (
                "/execution_metadata/expires_at_ms",
                json!(now + local_quote::LOCAL_QUOTE_SIGNING_WINDOW_MS),
            ),
            ("/snapshot_age_ms", json!(20001)),
            ("/execution_metadata/hops/0/min_return", json!("0")),
            ("/router_operations/0/terra_swap/min_return", json!("79")),
        ] {
            let mut invalid = body.clone();
            if path == "/execution_supported" {
                invalid["execution_supported"] = value;
            } else if path == "/execution_metadata/execution_supported" {
                invalid["execution_metadata"]["execution_supported"] = value;
            } else {
                *invalid.pointer_mut(path).unwrap() = value;
            }
            assert!(!executable(&invalid, &scenario, now), "{path}");
        }
    }
}
