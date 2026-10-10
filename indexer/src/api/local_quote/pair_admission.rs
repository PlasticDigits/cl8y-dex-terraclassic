//! Fixed-cardinality prerequisite failures. Successful negative/malformed reads
//! invalidate immediately; provider loss preserves only existing original TTLs.
use super::*;
use crate::api::local_hook_pricing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AdmissionFailure {
    Missing,
    Unsupported,
    Protected,
}
impl AdmissionFailure {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Unsupported => "unsupported",
            Self::Protected => "protected",
        }
    }
}
async fn required(
    state: &AppState,
    address: &str,
    query: Value,
) -> Result<Value, AdmissionFailure> {
    state
        .lcd
        .query_contract_bounded(address, &query, 32_768)
        .await
        .map_err(|error| {
            if crate::indexer::block_indexer::is_retryable_availability(&error) {
                AdmissionFailure::Missing
            } else {
                AdmissionFailure::Unsupported
            }
        })
}
fn allowed_bool(value: &Value, field: &str, expected: bool) -> Result<(), AdmissionFailure> {
    match value[field].as_bool() {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(AdmissionFailure::Protected),
        None => Err(AdmissionFailure::Unsupported),
    }
}
fn allowed_pins(value: &Value, codes: [u64; 2]) -> Result<(), AdmissionFailure> {
    let pins = value["code_ids"]
        .as_array()
        .ok_or(AdmissionFailure::Unsupported)?;
    if pins.len() != 2 || pins.iter().any(|pin| pin.as_u64().is_none()) {
        return Err(AdmissionFailure::Unsupported);
    }
    if value["code_ids"] != json!(codes) {
        return Err(AdmissionFailure::Protected);
    }
    Ok(())
}
fn allowed_blacklist(value: &Value) -> Result<(), AdmissionFailure> {
    for field in ["blacklisted_tokens", "blacklisted_pairs"] {
        let entries = value[field]
            .as_array()
            .ok_or(AdmissionFailure::Unsupported)?;
        if !entries.is_empty() {
            return Err(AdmissionFailure::Protected);
        }
    }
    // The complete current factory schema is authoritative, including its
    // aggregate guard. Missing or inconsistent fields never grant admission.
    for field in ["blocked", "wallet_blacklisted", "pair_blacklisted"] {
        allowed_bool(value, field, false)?;
    }
    Ok(())
}
pub(super) async fn observe_pair(
    state: &AppState,
    pair: &pairs::PairRow,
    a: &str,
    b: &str,
    code_a: u64,
    code_b: u64,
    factory: &str,
) -> Result<(local_hook_pricing::CachedHookPrices, bool, String), AdmissionFailure> {
    let paused = required(state, &pair.contract_address, json!({"is_paused":{}})).await?;
    allowed_bool(&paused, "paused", false)?;
    let pins = required(
        state,
        &pair.contract_address,
        json!({"get_asset_code_ids":{}}),
    )
    .await?;
    allowed_pins(&pins, [code_a, code_b])?;
    let blacklist = required(state, factory, json!({"blacklist_check":{"tokens":[a,b],"pairs":[pair.contract_address],"wallet":null,"pair":null}})).await?;
    allowed_blacklist(&blacklist)?;
    for code in [Some(code_a), (code_b != code_a).then_some(code_b)]
        .into_iter()
        .flatten()
    {
        let whitelist = required(
            state,
            factory,
            json!({"is_code_id_whitelisted":{"code_id":code}}),
        )
        .await?;
        allowed_bool(&whitelist, "whitelisted", true)?;
    }
    let hooks = required(state, &pair.contract_address, json!({"get_hooks":{}})).await?;
    let list = hooks["hooks"]
        .as_array()
        .ok_or(AdmissionFailure::Unsupported)?;
    if list.len() > 8 {
        return Err(AdmissionFailure::Unsupported);
    }
    let mut observations = Vec::new();
    for hook in list {
        let address = hook.as_str().ok_or(AdmissionFailure::Unsupported)?;
        observations.push((
            address.to_owned(),
            required(state, address, json!({"get_config":{}})).await?,
        ));
    }
    let prices = local_hook_pricing::CachedHookPrices::from_observation(&observations)
        .map_err(|_| AdmissionFailure::Unsupported)?;
    // Unavailable discounts use the conservative base fee and do not renew an
    // observation of any other required prerequisite.
    let registry = super::ingestion::query(
        state,
        &pair.contract_address,
        json!({"get_discount_registry":{}}),
    )
    .await;
    let discounted = state
        .fee_discount_address
        .as_deref()
        .is_some_and(|configured| {
            registry
                .as_ref()
                .and_then(|value| value["registry"].as_str())
                == Some(configured)
        });
    Ok((
        prices,
        discounted,
        format!("{}|{pins}|{registry:?}", pair.contract_address),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pair_schema_distinguishes_malformed_unsupported_from_authoritative_protection() {
        assert_eq!(
            allowed_bool(&json!({}), "paused", false),
            Err(AdmissionFailure::Unsupported)
        );
        assert_eq!(
            allowed_bool(&json!({"paused":true}), "paused", false),
            Err(AdmissionFailure::Protected)
        );
        assert!(allowed_pins(&json!({"code_ids":[18,6]}), [18, 6]).is_ok());
        assert_eq!(
            allowed_pins(&json!({"code_ids":[6,18]}), [18, 6]),
            Err(AdmissionFailure::Protected)
        );
        assert_eq!(
            allowed_pins(&json!({"code_ids":["18",6]}), [18, 6]),
            Err(AdmissionFailure::Unsupported)
        );
        assert_eq!(
            allowed_pins(&json!({"code_ids":[18,6,1]}), [18, 6]),
            Err(AdmissionFailure::Unsupported)
        );
        let mut valid = json!({"blocked":false,"wallet_blacklisted":false,"pair_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[]});
        assert!(allowed_blacklist(&valid).is_ok());
        valid["blocked"] = json!(true);
        assert_eq!(allowed_blacklist(&valid), Err(AdmissionFailure::Protected));
        assert_eq!(
            allowed_blacklist(&json!({"blacklisted_tokens":[],"blacklisted_pairs":[]})),
            Err(AdmissionFailure::Unsupported)
        );
    }
}
