//! Pinned factory creation fees and paid community invoices (#1209/#1210/#1354).
//! Each authenticated action reserves its replay ordinal before amount validation.
//! Token invoices require an anchored launcher receipt, code/admin attestation and
//! an exact UST1 contract identity; self-reported launcher origin is insufficient.

use bigdecimal::BigDecimal;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use super::protocol_fees::{
    positive_fee_amount, wasm_contract_addr_before, wasm_kv_map_after_action, FeeSource,
    ParsedWrapFee,
};
use crate::{
    config::Config,
    db::queries::community_tokens,
    lcd::{LcdClient, LcdError, TxResponse},
};

pub struct InvoiceCandidate {
    pub emitter: String,
    pub fee: ParsedWrapFee,
}

/// Successful receipts only; logs win over the duplicate top-level event list.
pub fn parse_invoice_fees(
    tx: &TxResponse,
    factory: &str,
    launcher: &str,
    ust1: &str,
) -> Vec<InvoiceCandidate> {
    if tx.code != 0 {
        return Vec::new();
    }
    let events: Vec<_> = match &tx.logs {
        Some(logs) => logs.iter().flat_map(|l| &l.events).collect(),
        None => tx.events.iter().flatten().collect(),
    };
    let mut ordinals = std::collections::HashMap::<FeeSource, i64>::new();
    let mut out = Vec::new();
    for event in events {
        if event.event_type != "wasm" && event.event_type != "wasm-wasm" {
            continue;
        }
        for (i, action) in event.attributes.iter().enumerate() {
            if action.key != "action" {
                continue;
            }
            let Some(emitter) = wasm_contract_addr_before(&event.attributes, i) else {
                continue;
            };
            let source = match action.value.as_str() {
                "create_pair" if !factory.is_empty() && emitter == factory => {
                    FeeSource::PairCreation
                }
                "create_token"
                    if !launcher.is_empty() && emitter == launcher && !ust1.is_empty() =>
                {
                    FeeSource::SkuUnlock
                }
                // The launcher segment is only a forwarding leg, never a paid invoice.
                "enable_feature"
                    if !launcher.is_empty() && emitter != launcher && !ust1.is_empty() =>
                {
                    FeeSource::SkuUnlock
                }
                "update_settings"
                    if !launcher.is_empty() && emitter != launcher && !ust1.is_empty() =>
                {
                    FeeSource::SettingsFee
                }
                _ => continue,
            };
            let next = ordinals.entry(source).or_default();
            let ordinal = *next;
            *next += 1;
            let segment = wasm_kv_map_after_action(&event.attributes, i);
            let amount = if action.value == "create_token" {
                positive_fee_amount(segment.get("sku_count").copied(), source, "sku_count")
                    .filter(|count| count <= &BigDecimal::from(7))
                    .map(|count| count * BigDecimal::from(50_000_000))
            } else {
                let field = if source == FeeSource::PairCreation {
                    "creation_fee_uluna"
                } else {
                    "invoice"
                };
                positive_fee_amount(segment.get(field).copied(), source, field)
            };
            let Some(amount_raw) = amount else {
                continue;
            };
            out.push(InvoiceCandidate {
                emitter: emitter.to_owned(),
                fee: ParsedWrapFee {
                    source,
                    amount_raw,
                    token: if source == FeeSource::PairCreation {
                        "uluna"
                    } else {
                        ust1
                    }
                    .to_owned(),
                    ordinal,
                },
            });
        }
    }
    out
}

#[derive(Deserialize)]
struct InvoiceConfig {
    ust1: String,
    cmm_treasury: String,
    token_code_id: Option<u64>,
    cmm_governance: Option<String>,
}

/// Verify the actual payment contract from trusted on-chain config. A catalog
/// entry created by GetLauncherOrigin or migrate-adopt alone cannot admit a fee.
pub async fn trusted_invoice_emitter(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    candidate: &InvoiceCandidate,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if candidate.fee.source == FeeSource::PairCreation {
        return Ok(true);
    }
    let Some(launcher) = config.community_token_launcher.as_deref() else {
        return Ok(false);
    };
    if candidate.emitter != launcher {
        let Some(row) = community_tokens::get_by_address(pool, &candidate.emitter).await? else {
            return Ok(false);
        };
        if !row.attested_cmm
            || row.launcher_address.as_deref() != Some(launcher)
            || !row.launcher_tx.as_ref().is_some_and(|tx| !tx.is_empty())
            || row.code_id.and_then(|id| u64::try_from(id).ok()) != config.community_tax_code_id
            || row.wasm_admin.as_deref() != config.cmm_governance_addr.as_deref()
            || config.community_tax_code_id.is_none()
            || config.cmm_governance_addr.is_none()
        {
            return Ok(false);
        }
    }
    let Some(launcher_config) = load_invoice_config(lcd, launcher).await? else {
        return Ok(false);
    };
    if launcher_config.ust1 != config.hub_ust1_address
        || super::protocol_fees::parse_terra1_pin(&launcher_config.cmm_treasury).is_none()
        || launcher_config.token_code_id != config.community_tax_code_id
        || launcher_config.cmm_governance.as_deref() != config.cmm_governance_addr.as_deref()
        || config.community_tax_code_id.is_none()
        || config.cmm_governance_addr.is_none()
    {
        return Ok(false);
    }
    if candidate.emitter == launcher {
        return Ok(true);
    }
    let queried = load_invoice_config(lcd, &candidate.emitter).await?;
    Ok(queried.is_some_and(|queried| {
        queried.ust1 == launcher_config.ust1 && queried.cmm_treasury == launcher_config.cmm_treasury
    }))
}

/// Infrastructure failures must retry the block instead of silently consuming
/// its height. Unsupported/malformed contract responses cannot establish trust.
async fn load_invoice_config(
    lcd: &LcdClient,
    emitter: &str,
) -> Result<Option<InvoiceConfig>, Box<dyn std::error::Error + Send + Sync>> {
    match lcd
        .query_contract::<InvoiceConfig>(emitter, &json!({"get_config": {}}))
        .await
    {
        Ok(config) => Ok(Some(config)),
        Err(LcdError::ContractQueryRejected(_) | LcdError::Deserialize(_)) => Ok(None),
        Err(error) => Err(Box::new(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcd::{Attribute, Event};

    fn tx(attrs: &[(&str, &str)]) -> TxResponse {
        TxResponse {
            code: 0,
            height: "1".into(),
            txhash: "tx".into(),
            timestamp: None,
            logs: None,
            events: Some(vec![Event {
                event_type: "wasm".into(),
                attributes: attrs
                    .iter()
                    .map(|(k, v)| Attribute {
                        key: k.to_string(),
                        value: v.to_string(),
                    })
                    .collect(),
            }]),
        }
    }
    #[test]
    fn factory_positive_emitted_only_and_stable_ordinal() {
        let receipt = tx(&[
            ("_contract_address", "factory"),
            ("action", "create_pair"),
            ("creation_fee_uluna", "bad"),
            ("action", "create_pair"),
            ("creation_fee_uluna", "90"),
            ("action", "reply_instantiate_pair"),
            ("creation_fee_uluna", "99"),
            ("_contract_address", "spoof"),
            ("action", "create_pair"),
            ("creation_fee_uluna", "88"),
        ]);
        let fees = parse_invoice_fees(&receipt, "factory", "launcher", "ust1");
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].fee.amount_raw, BigDecimal::from(90));
        assert_eq!(fees[0].fee.ordinal, 1);
        assert_eq!(fees[0].fee.token, "uluna");
    }
    #[test]
    fn invoices_scope_segments_and_exclude_forwarding_and_ready() {
        let receipt = tx(&[
            ("_contract_address", "launcher"),
            ("action", "create_token"),
            ("sku_count", "2"),
            ("action", "create_token_ready"),
            ("invoice", "100000000"),
            ("action", "enable_feature"),
            ("token", "token"),
            ("invoice", "50000000"),
            ("_contract_address", "token"),
            ("action", "enable_feature"),
            ("invoice", "50000000"),
            ("action", "update_settings"),
            ("invoice", "50000000"),
            ("fields", "3"),
        ]);
        let fees = parse_invoice_fees(&receipt, "factory", "launcher", "ust1");
        assert_eq!(fees.len(), 3);
        assert_eq!(fees[0].fee.amount_raw, BigDecimal::from(100_000_000));
        assert_eq!(fees[1].fee.ordinal, 1);
        assert_eq!(fees[2].fee.source, FeeSource::SettingsFee);
        assert_eq!(fees[2].fee.amount_raw, BigDecimal::from(50_000_000));
    }
    #[test]
    fn rejects_missing_pins_unreserved_emitters_reverts_and_bad_amounts() {
        for value in ["0", "-1", "1.2", "1e2", "bad", ""] {
            assert!(parse_invoice_fees(
                &tx(&[
                    ("_contract_address", "launcher"),
                    ("action", "create_token"),
                    ("sku_count", value)
                ]),
                "factory",
                "launcher",
                "ust1"
            )
            .is_empty());
        }
        for action in ["create_pair", "enable_feature", "update_settings"] {
            for value in ["0", "-1", "1.2", "1e2", "bad", ""] {
                let field = if action == "create_pair" {
                    "creation_fee_uluna"
                } else {
                    "invoice"
                };
                let emitter = if action == "create_pair" {
                    "factory"
                } else {
                    "token"
                };
                assert!(parse_invoice_fees(
                    &tx(&[
                        ("_contract_address", emitter),
                        ("action", action),
                        (field, value)
                    ]),
                    "factory",
                    "launcher",
                    "ust1"
                )
                .is_empty());
            }
        }
        assert!(parse_invoice_fees(
            &tx(&[
                ("_contract_address", "launcher"),
                ("action", "create_token"),
                ("sku_count", "8")
            ]),
            "factory",
            "launcher",
            "ust1"
        )
        .is_empty());
        let mut receipt = tx(&[
            ("contract_address", "factory"),
            ("action", "create_pair"),
            ("creation_fee_uluna", "50"),
        ]);
        assert!(parse_invoice_fees(&receipt, "factory", "launcher", "ust1").is_empty());
        receipt = tx(&[
            ("_contract_address", "factory"),
            ("action", "create_pair"),
            ("creation_fee_uluna", "50"),
        ]);
        assert!(parse_invoice_fees(&receipt, "", "", "ust1").is_empty());
        receipt.code = 5;
        assert!(parse_invoice_fees(&receipt, "factory", "launcher", "ust1").is_empty());
    }
}
