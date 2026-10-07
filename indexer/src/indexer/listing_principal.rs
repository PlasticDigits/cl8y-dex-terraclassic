//! CG/GT-only principal projection. No writes to economic pair, volume or fee tables.
use std::collections::HashMap;

use base64::Engine;
use bech32::FromBase32;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    indexer::event_amount::{parse_raw_amount, report_amount_rejection},
    lcd::{LcdClient, TxResponse},
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ParsedPrincipal {
    pub native_denom: String,
    pub direction: &'static str,
    pub principal_raw: BigDecimal,
    /// Actual treasury bank movement: gross deposit, or principal withdrawal.
    /// Mint/burn fees remain outside projected trade amounts.
    pub custody_delta: BigDecimal,
    pub trader_address: String,
    pub source_ordinal: i64,
}

/// Select one LCD representation: legacy logs take precedence when they contain events.
/// SDK responses with empty logs may instead carry their events at the top level.
fn tx_events(tx: &TxResponse) -> Vec<&crate::lcd::Event> {
    let logged: Vec<_> = tx
        .logs
        .iter()
        .flatten()
        .flat_map(|log| &log.events)
        .collect();
    if logged.is_empty() {
        tx.events.iter().flatten().collect()
    } else {
        logged
    }
}

/// Sender comes from SDK message events, never a mapper's recipient/depositor attribute.
fn transaction_sender(events: &[&crate::lcd::Event]) -> Option<String> {
    events
        .iter()
        .copied()
        .filter(|e| e.event_type == "message")
        .flat_map(|e| &e.attributes)
        .find(|a| a.key == "sender" && !a.value.is_empty())
        .map(|a| a.value.clone())
}

pub fn parse_principal(tx: &TxResponse, mapper: &str) -> Vec<ParsedPrincipal> {
    if mapper.is_empty() || tx.code != 0 {
        return Vec::new();
    }
    let events = tx_events(tx);
    let Some(sender) = transaction_sender(&events) else {
        return Vec::new();
    };
    let mut ordinal = 0;
    let mut result = Vec::new();
    for event in events {
        if !matches!(event.event_type.as_str(), "wasm" | "wasm-wasm") {
            continue;
        }
        for (index, attr) in event.attributes.iter().enumerate() {
            if attr.key != "action" {
                continue;
            }
            let source_ordinal = ordinal;
            ordinal += 1;
            let direction = match attr.value.as_str() {
                "notify_deposit" => "wrap",
                "unwrap" => "unwrap",
                _ => continue,
            };
            let contract = event.attributes[..index]
                .iter()
                .rev()
                .find(|a| a.key == "_contract_address")
                .map(|a| a.value.as_str());
            if contract != Some(mapper) {
                continue;
            }
            let mut fields = HashMap::new();
            let mut duplicate = false;
            for attr in &event.attributes[index + 1..] {
                if matches!(attr.key.as_str(), "action" | "_contract_address") {
                    break;
                }
                if fields
                    .insert(attr.key.as_str(), attr.value.as_str())
                    .is_some()
                {
                    duplicate = true;
                }
            }
            if duplicate {
                continue;
            }
            let Some(&denom @ ("uluna" | "uusd")) = fields.get("denom") else {
                continue;
            };
            let field = if direction == "wrap" {
                "mint_amount"
            } else {
                "withdraw_amount"
            };
            let principal = match parse_raw_amount(fields.get(field).copied()) {
                Ok(value) if value > BigDecimal::from(0) => value,
                Ok(_) => continue,
                Err(error) => {
                    report_amount_rejection("listing_principal", field, error);
                    continue;
                }
            };
            let gross = match parse_raw_amount(fields.get("gross_amount").copied()) {
                Ok(gross) if gross >= principal => gross,
                Ok(_) => continue,
                Err(error) => {
                    report_amount_rejection("listing_principal", "gross_amount", error);
                    continue;
                }
            };
            let custody_delta = if direction == "wrap" {
                gross
            } else {
                -principal.clone()
            };
            result.push(ParsedPrincipal {
                native_denom: denom.to_owned(),
                direction,
                principal_raw: principal,
                custody_delta,
                trader_address: sender.clone(),
                source_ordinal,
            });
        }
    }
    result
}

#[derive(Deserialize)]
struct BankBalance {
    balance: Option<BankCoin>,
}
#[derive(Deserialize)]
struct BankCoin {
    denom: String,
    amount: String,
}

/// Accept only a canonical Terra contract address before interpolating an LCD path.
fn canonical_custody_address(address: &str) -> bool {
    address.len() <= 90
        && address.starts_with("terra1")
        && address.bytes().all(|b| !b.is_ascii_uppercase())
        && bech32::decode(address)
            .ok()
            .is_some_and(|(hrp, data, variant)| {
                hrp == "terra"
                    && variant == bech32::Variant::Bech32
                    && Vec::<u8>::from_base32(&data)
                        .ok()
                        .is_some_and(|bytes| matches!(bytes.len(), 20 | 32))
            })
}

#[derive(Deserialize)]
struct MapperConfigResponse {
    data: MapperConfig,
}

#[derive(Deserialize)]
struct MapperConfig {
    treasury: String,
}

async fn treasury_at(lcd: &LcdClient, mapper: &str, height: i64) -> Result<String, BoxError> {
    if !canonical_custody_address(mapper) {
        return Err("invalid mapper contract address".into());
    }
    let query = base64::engine::general_purpose::STANDARD.encode(br#"{"config":{}}"#);
    let response: MapperConfigResponse = lcd
        .get_at_height(
            &format!("/cosmwasm/wasm/v1/contract/{mapper}/smart/{query}"),
            height,
        )
        .await?;
    if !canonical_custody_address(&response.data.treasury) {
        return Err("invalid mapper treasury contract address".into());
    }
    Ok(response.data.treasury)
}

/// Reconciliation spans the source block and predecessor. A custody rotation
/// cannot combine balances from different authoritative treasury contracts.
async fn verified_treasury(lcd: &LcdClient, mapper: &str, height: i64) -> Result<String, BoxError> {
    let current = treasury_at(lcd, mapper, height).await?;
    let previous = treasury_at(lcd, mapper, height - 1).await?;
    if current != previous {
        return Err("mapper treasury changed across source block".into());
    }
    Ok(current)
}

async fn custody_at(
    lcd: &LcdClient,
    custody_address: &str,
    denom: &str,
    height: i64,
) -> Result<BigDecimal, BoxError> {
    let response: BankBalance = lcd
        .get_at_height(
            &format!("/cosmos/bank/v1beta1/balances/{custody_address}/by_denom?denom={denom}"),
            height,
        )
        .await?;
    match response.balance {
        Some(coin) if coin.denom == denom => Ok(parse_raw_amount(Some(&coin.amount))?),
        Some(_) => Err("bank custody denomination mismatch".into()),
        None => Ok(BigDecimal::from(0)),
    }
}

struct Snapshot<'a> {
    mapper: &'a str,
    custody_address: Option<&'a str>,
    denom: &'a str,
    custody: &'a BigDecimal,
    height: i64,
    timestamp: DateTime<Utc>,
    block_hash: &'a str,
}

async fn persist_snapshot(
    executor: &mut sqlx::PgConnection,
    snapshot: Snapshot<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO listing_mapper_custody(mapper_address,custody_address,native_denom,custody_raw,block_height,block_timestamp,block_hash) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(mapper_address,native_denom,block_height) DO UPDATE SET custody_address=EXCLUDED.custody_address,custody_raw=EXCLUDED.custody_raw,block_timestamp=EXCLUDED.block_timestamp,block_hash=EXCLUDED.block_hash")
        .bind(snapshot.mapper).bind(snapshot.custody_address).bind(snapshot.denom).bind(snapshot.custody)
        .bind(snapshot.height).bind(snapshot.timestamp).bind(snapshot.block_hash)
        .execute(executor).await?;
    Ok(())
}

/// An unavailable proof replaces the current snapshot with an explicit unknown
/// source. The adapter rejects it instead of falling back to older positive custody.
async fn persist_unproven(
    pool: &PgPool,
    mapper: &str,
    denoms: &[&str],
    height: i64,
    timestamp: DateTime<Utc>,
    block_hash: &str,
) -> Result<(), sqlx::Error> {
    let mut db = pool.begin().await?;
    for denom in denoms {
        persist_snapshot(
            &mut db,
            Snapshot {
                mapper,
                custody_address: None,
                denom,
                custody: &BigDecimal::from(0),
                height,
                timestamp,
                block_hash,
            },
        )
        .await?;
    }
    db.commit().await?;
    Ok(())
}

/// Reconstruct post-action backing from the historical bank balance and actual custody deltas.
/// Missing/pruned snapshots fail closed for the listing projection without blocking economic indexing.
pub async fn ingest_block(
    pool: &PgPool,
    lcd: &LcdClient,
    mapper: &str,
    txs: &[TxResponse],
    height: i64,
    timestamp: DateTime<Utc>,
    block_hash: &str,
) -> Result<(), BoxError> {
    let parsed: Vec<_> = txs
        .iter()
        .enumerate()
        .flat_map(|(tx_index, tx)| {
            let events = if tx.height.parse::<i64>().ok() == Some(height) {
                parse_principal(tx, mapper)
            } else {
                Vec::new()
            };
            events
                .into_iter()
                .map(move |event| (tx_index as i64, &tx.txhash, event))
        })
        .collect();
    if parsed.is_empty() {
        return Ok(());
    }
    let treasury = match verified_treasury(lcd, mapper, height).await {
        Ok(address) => address,
        Err(_) => {
            persist_unproven(
                pool,
                mapper,
                &["uluna", "uusd"],
                height,
                timestamp,
                block_hash,
            )
            .await?;
            tracing::warn!(
                event_kind = "listing_principal",
                "Historical treasury identity unavailable; projection omitted"
            );
            return Ok(());
        }
    };
    for denom in ["uluna", "uusd"] {
        let matching: Vec<_> = parsed
            .iter()
            .filter(|(_, _, e)| e.native_denom == denom)
            .collect();
        if matching.is_empty() {
            continue;
        }
        let final_custody = match custody_at(lcd, &treasury, denom, height).await {
            Ok(value) => value,
            Err(_) => {
                tracing::warn!(
                    event_kind = "listing_principal",
                    "Historical treasury custody unavailable; projection omitted"
                );
                persist_unproven(pool, mapper, &[denom], height, timestamp, block_hash).await?;
                continue;
            }
        };
        let mut custody = final_custody.clone();
        let mut rows = Vec::new();
        let mut valid = true;
        for (tx_index, hash, event) in matching.into_iter().rev() {
            if custody < BigDecimal::from(0) {
                valid = false;
                break;
            }
            rows.push((*tx_index, *hash, event, custody.clone()));
            custody -= &event.custody_delta;
        }
        if !valid || custody < BigDecimal::from(0) {
            persist_unproven(pool, mapper, &[denom], height, timestamp, block_hash).await?;
            continue;
        }
        // Reconcile the complete block so unrelated native transfers cannot manufacture reserves.
        match custody_at(lcd, &treasury, denom, height - 1).await {
            Ok(initial) if initial == custody => {}
            _ => {
                tracing::warn!(
                    event_kind = "listing_principal",
                    "Treasury custody movement cannot be reconciled; projection omitted"
                );
                persist_unproven(pool, mapper, &[denom], height, timestamp, block_hash).await?;
                continue;
            }
        }
        let mut db = pool.begin().await?;
        persist_snapshot(
            &mut db,
            Snapshot {
                mapper,
                custody_address: Some(&treasury),
                denom,
                custody: &final_custody,
                height,
                timestamp,
                block_hash,
            },
        )
        .await?;
        for (tx_index, hash, event, custody) in rows.into_iter().rev() {
            sqlx::query("INSERT INTO listing_wrap_principal(mapper_address,custody_address,native_denom,direction,principal_raw,custody_raw,tx_hash,trader_address,source_ordinal,txn_index,block_height,block_timestamp,block_hash) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) ON CONFLICT(mapper_address,tx_hash,source_ordinal) DO NOTHING")
                .bind(mapper).bind(&treasury).bind(denom).bind(event.direction).bind(&event.principal_raw).bind(custody)
                .bind(hash).bind(&event.trader_address).bind(event.source_ordinal).bind(tx_index)
                .bind(height).bind(timestamp).bind(block_hash).execute(&mut *db).await?;
        }
        db.commit().await?;
    }
    Ok(())
}

/// Bootstrap backing from the indexed checkpoint. This runs outside the request path.
pub async fn refresh_custody(pool: &PgPool, lcd: &LcdClient, mapper: &str) -> Result<(), BoxError> {
    let (height, hash) = crate::db::queries::state::get_indexer_checkpoint(pool).await?;
    if height <= 0 {
        return Ok(());
    }
    let block = lcd.get_block_at_height(height).await?;
    if block.block_id.as_ref().map(|id| id.hash.as_str()) != hash.as_deref() {
        return Ok(());
    }
    let timestamp = DateTime::parse_from_rfc3339(&block.block.header.time)?.with_timezone(&Utc);
    let treasury = match verified_treasury(lcd, mapper, height).await {
        Ok(address) => address,
        Err(error) => {
            persist_unproven(
                pool,
                mapper,
                &["uluna", "uusd"],
                height,
                timestamp,
                hash.as_deref().unwrap_or(""),
            )
            .await?;
            return Err(error);
        }
    };
    let mut unavailable = false;
    for denom in ["uluna", "uusd"] {
        match custody_at(lcd, &treasury, denom, height).await {
            Ok(custody) => {
                let mut db = pool.begin().await?;
                persist_snapshot(
                    &mut db,
                    Snapshot {
                        mapper,
                        custody_address: Some(&treasury),
                        denom,
                        custody: &custody,
                        height,
                        timestamp,
                        block_hash: hash.as_deref().unwrap_or(""),
                    },
                )
                .await?;
                db.commit().await?;
            }
            Err(_) => {
                persist_unproven(
                    pool,
                    mapper,
                    &[denom],
                    height,
                    timestamp,
                    hash.as_deref().unwrap_or(""),
                )
                .await?;
                unavailable = true;
            }
        }
    }
    if unavailable {
        return Err("Treasury custody unavailable at indexed checkpoint".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcd::{Attribute, Event};

    fn tx(mapper: &str, action: &str, principal: &str) -> TxResponse {
        let principal_field = if action == "unwrap" {
            "withdraw_amount"
        } else {
            "mint_amount"
        };
        let attrs = [
            ("_contract_address", mapper),
            ("action", action),
            ("denom", "uluna"),
            ("gross_amount", "5010000"),
            ("fee", "10000"),
            (principal_field, principal),
        ];
        TxResponse {
            code: 0,
            height: "10".into(),
            txhash: "CHAIN_TX".into(),
            logs: None,
            timestamp: None,
            events: Some(vec![
                Event {
                    event_type: "message".into(),
                    attributes: vec![Attribute {
                        key: "sender".into(),
                        value: "actual_sender".into(),
                    }],
                },
                Event {
                    event_type: "wasm".into(),
                    attributes: attrs
                        .into_iter()
                        .map(|(key, value)| Attribute {
                            key: key.into(),
                            value: value.into(),
                        })
                        .collect(),
                },
            ]),
        }
    }

    #[test]
    fn pinned_principal_excludes_fee_and_preserves_sender() {
        for action in ["notify_deposit", "unwrap"] {
            let events = parse_principal(&tx("pinned", action, "5000000"), "pinned");
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].principal_raw, BigDecimal::from(5_000_000));
            assert_eq!(events[0].trader_address, "actual_sender");
            assert_eq!(
                events[0].custody_delta,
                BigDecimal::from(if action == "unwrap" {
                    -5_000_000
                } else {
                    5_010_000
                })
            );
        }
    }

    #[test]
    fn forged_contract_missing_sender_zero_and_malformed_principal_fail_closed() {
        let mut failed = tx("pinned", "notify_deposit", "5000000");
        failed.code = 5;
        assert!(parse_principal(&failed, "pinned").is_empty());
        assert!(parse_principal(&tx("foreign", "notify_deposit", "5000000"), "pinned").is_empty());
        assert!(parse_principal(&tx("pinned", "notify_deposit", "5000000"), "").is_empty());
        for principal in [
            "0",
            "-1",
            "1e6",
            " 5",
            "1.0",
            "999999999999999999999999999999999999999",
        ] {
            assert!(
                parse_principal(&tx("pinned", "notify_deposit", principal), "pinned").is_empty()
            );
        }
        let mut missing = tx("pinned", "notify_deposit", "5000000");
        missing.events.as_mut().unwrap().remove(0);
        assert!(parse_principal(&missing, "pinned").is_empty());
        let mut spoofed = tx("foreign", "notify_deposit", "5000000");
        spoofed.events.as_mut().unwrap()[1].attributes[0].key = "contract_address".into();
        assert!(parse_principal(&spoofed, "pinned").is_empty());
    }

    #[test]
    fn rejection_keeps_source_ordinal_and_prefers_logs_over_duplicate_events() {
        let mut event_tx = tx("pinned", "notify_deposit", "0");
        let valid = tx("pinned", "notify_deposit", "5000000");
        event_tx
            .events
            .as_mut()
            .unwrap()
            .push(valid.events.unwrap().remove(1));
        let parsed = parse_principal(&event_tx, "pinned");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].source_ordinal, 1);
        event_tx.logs = Some(vec![crate::lcd::TxLog {
            events: event_tx.events.clone().unwrap(),
        }]);
        assert_eq!(parse_principal(&event_tx, "pinned").len(), 1);
    }

    #[test]
    fn empty_logs_fall_back_to_top_level_sdk_events() {
        for logs in [serde_json::json!([]), serde_json::json!([{ "events": [] }])] {
            let tx: TxResponse = serde_json::from_value(serde_json::json!({
                "code": 0, "height": "10", "txhash": "REAL_TX", "logs": logs,
                "events": [
                    {"type": "message", "attributes": [{"key": "sender", "value": "sdk_sender"}]},
                    {"type": "wasm", "attributes": [
                        {"key": "_contract_address", "value": "pinned"},
                        {"key": "action", "value": "notify_deposit"},
                        {"key": "denom", "value": "uluna"},
                        {"key": "gross_amount", "value": "5010000"},
                        {"key": "mint_amount", "value": "5000000"}
                    ]}
                ]
            }))
            .unwrap();
            let events = parse_principal(&tx, "pinned");
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].trader_address, "sdk_sender");
            assert_eq!(events[0].principal_raw, BigDecimal::from(5_000_000));
        }
    }

    #[test]
    fn gross_bank_movements_are_strict_and_distinct_from_trade_principal() {
        for action in ["notify_deposit", "unwrap"] {
            for gross in [
                "0",
                "4999999",
                "-1",
                "1e6",
                "1.0",
                " 5010000",
                "999999999999999999999999999999999999999",
            ] {
                let mut source = tx("pinned", action, "5000000");
                let field = source.events.as_mut().unwrap()[1]
                    .attributes
                    .iter_mut()
                    .find(|a| a.key == "gross_amount")
                    .unwrap();
                field.value = gross.into();
                assert!(parse_principal(&source, "pinned").is_empty());
            }
            let mut missing = tx("pinned", action, "5000000");
            missing.events.as_mut().unwrap()[1]
                .attributes
                .retain(|a| a.key != "gross_amount");
            assert!(parse_principal(&missing, "pinned").is_empty());
        }
    }

    #[test]
    fn custody_contract_addresses_require_canonical_terra_checksums() {
        use bech32::ToBase32;
        for size in [20, 32] {
            let address =
                bech32::encode("terra", vec![7; size].to_base32(), bech32::Variant::Bech32)
                    .unwrap();
            assert!(canonical_custody_address(&address));
            assert!(!canonical_custody_address(&address.to_ascii_uppercase()));
            assert!(!canonical_custody_address(&format!(" {address}")));
        }
        for address in [
            "terra1notchecksummed",
            "terra1/address",
            "https://treasury",
            "cl8y-virtual:treasury",
        ] {
            assert!(!canonical_custody_address(address));
        }
        let other =
            bech32::encode("osmo", vec![7; 32].to_base32(), bech32::Variant::Bech32).unwrap();
        assert!(!canonical_custody_address(&other));
        let wrong_variant =
            bech32::encode("terra", vec![7; 32].to_base32(), bech32::Variant::Bech32m).unwrap();
        assert!(!canonical_custody_address(&wrong_variant));
        let short =
            bech32::encode("terra", vec![7; 4].to_base32(), bech32::Variant::Bech32).unwrap();
        assert!(!canonical_custody_address(&short));
    }

    #[tokio::test]
    async fn bank_custody_rejects_wrong_denom_negative_and_missing_height_attestation() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for (response_header, denom, amount, accepted) in [
            (Some("10"), "uluna", "5000000", true),
            (None, "uluna", "5000000", false),
            (Some("11"), "uluna", "5000000", false),
            (Some("10"), "uusd", "5000000", false),
            (Some("10"), "uluna", "-1", false),
        ] {
            let server = MockServer::start().await;
            crate::lcd::mock_chain_identity(&server, "columbus-5").await;
            let mut response = ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"balance":{"denom":denom,"amount":amount}}));
            if let Some(value) = response_header {
                response = response.insert_header("x-cosmos-block-height", value);
            }
            Mock::given(method("GET"))
                .and(path("/cosmos/bank/v1beta1/balances/pinned/by_denom"))
                .and(header("x-cosmos-block-height", "10"))
                .respond_with(response)
                .expect(1)
                .mount(&server)
                .await;
            let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
            assert_eq!(
                custody_at(&lcd, "pinned", "uluna", 10).await.is_ok(),
                accepted
            );
        }
    }
}
