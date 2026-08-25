//! Protocol treasury fee ingest + wrap-mapper / ust1-window pins (GitLab #586 / #614).
//!
//! **PFee / L7:** count amounts sent to pair / wrap-mapper / ust1-window treasury only.
//! Do not count `spread_amount`, Terra Classic burn tax, gas, `hook_fee_amount`,
//! LP, book escrow, parked dust, Venus redeem, oracle spread, or unused rolling capacity.
//! Hybrid book taker is `limit_order_fills.commission_amount`
//! — never also add a swap-level `book_commission_amount`.
//!
//! Wrap/unwrap wasm is accepted only from a pinned `WRAP_MAPPER_ADDRESS` (exact bech32).
//! UST1 mint/redeem wasm is accepted only from a pinned `UST1_WINDOW_ADDRESS`.
//! Fail closed on missing treasury `fee` / `fee_amount` / token identity. Do not infer
//! `amount × bps` (including `ust1_out × fee_total_bps` / `vfdusd_to_treasury × fee_cmm_protocol_bps`).
//!
//! **Captured ustr-cmm wrap-mapper attrs (GitLab #613):** retail wrap is mapper
//! `action=notify_deposit` (treasury `wrap_deposit` is not a fee). Unwrap is
//! `action=unwrap`. The amount key is **`fee`** (not `fee_amount`). Token is `denom`
//! (`uusd` / `uluna`). LCD may flatten wrap + swap + InstantWithdraw into one `wasm`
//! stream — parse **each** `action` segment and scope by reserved `_contract_address`
//! only (#285). `tax_amount` / `instant_withdraw` / `wrap_deposit` are not fees.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};

use crate::db::queries::assets::AssetRow;
use crate::lcd::{Attribute, TxResponse};

use super::pair_price_usd::{
    catalog_usd_per_human_identity, fits_numeric_38_18, humanize_raw_amount, HubQuoteUsd,
};

/// Canonical fee source enum. Unknown DB strings must not be interpolated into JS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeeSource {
    SwapAmm,
    BookTake,
    LimitPlace,
    Wrap,
    Unwrap,
    Ust1Mint,
    Ust1Redeem,
}

impl FeeSource {
    pub const ALL: [FeeSource; 7] = [
        FeeSource::SwapAmm,
        FeeSource::BookTake,
        FeeSource::LimitPlace,
        FeeSource::Wrap,
        FeeSource::Unwrap,
        FeeSource::Ust1Mint,
        FeeSource::Ust1Redeem,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FeeSource::SwapAmm => "swap_amm",
            FeeSource::BookTake => "book_take",
            FeeSource::LimitPlace => "limit_place",
            FeeSource::Wrap => "wrap",
            FeeSource::Unwrap => "unwrap",
            FeeSource::Ust1Mint => "ust1_mint",
            FeeSource::Ust1Redeem => "ust1_redeem",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "swap_amm" => Some(FeeSource::SwapAmm),
            "book_take" => Some(FeeSource::BookTake),
            "limit_place" => Some(FeeSource::LimitPlace),
            "wrap" => Some(FeeSource::Wrap),
            "unwrap" => Some(FeeSource::Unwrap),
            "ust1_mint" => Some(FeeSource::Ust1Mint),
            "ust1_redeem" => Some(FeeSource::Ust1Redeem),
            _ => None,
        }
    }

    pub fn is_wrap_family(self) -> bool {
        matches!(self, FeeSource::Wrap | FeeSource::Unwrap)
    }

    pub fn is_ust1_window_family(self) -> bool {
        matches!(self, FeeSource::Ust1Mint | FeeSource::Ust1Redeem)
    }
}

/// Retail source labels (not wasm action strings).
pub fn fee_source_label(source: FeeSource) -> &'static str {
    match source {
        FeeSource::SwapAmm => "AMM swap",
        FeeSource::BookTake => "Book take",
        FeeSource::LimitPlace => "Limit place",
        FeeSource::Wrap => "Wrap",
        FeeSource::Unwrap => "Unwrap",
        FeeSource::Ust1Mint => "UST1 mint",
        FeeSource::Ust1Redeem => "UST1 redeem",
    }
}

/// Parsed wrap-mapper treasury fee. Token is a native denom or CW20 contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedWrapFee {
    pub source: FeeSource,
    pub amount_raw: BigDecimal,
    /// Native denom (`uusd`) or CW20 `terra1…`.
    pub token: String,
    pub ordinal: i64,
}

/// Pin a reserved `terra1` bech32: trim, reject whitespace/newline/wrong HRP, no `LIKE %`.
///
/// Shared by `WRAP_MAPPER_ADDRESS` and `UST1_WINDOW_ADDRESS`. Empty / garbage → `None`
/// (that source family omitted, not fake idle `$0`).
pub fn parse_terra1_pin(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    if !trimmed.starts_with("terra1") {
        return None;
    }
    if trimmed.len() < 20 || trimmed.len() > 90 {
        return None;
    }
    if !trimmed
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'z'))
    {
        return None;
    }
    Some(trimmed.to_string())
}

/// Pin `WRAP_MAPPER_ADDRESS` (GitLab #586).
pub fn parse_wrap_mapper_address(raw: &str) -> Option<String> {
    parse_terra1_pin(raw)
}

/// Pin `UST1_WINDOW_ADDRESS` (GitLab #614). Same rules as the wrap pin.
pub fn parse_ust1_window_address(raw: &str) -> Option<String> {
    parse_terra1_pin(raw)
}

/// Exact bech32 equality — never substring / case-fold.
pub fn wrap_mapper_matches(pinned: &str, contract: &str) -> bool {
    pin_matches(pinned, contract)
}

fn pin_matches(pinned: &str, contract: &str) -> bool {
    !pinned.is_empty() && pinned == contract
}

/// Same contract as parser lifecycle scans (#141 / #285): last reserved
/// `_contract_address` **before** `idx`. Forged `contract_address` is ignored.
fn wasm_contract_addr_before<'a>(attrs: &'a [Attribute], idx: usize) -> Option<&'a str> {
    attrs[..idx]
        .iter()
        .rev()
        .find(|a| a.key == "_contract_address")
        .map(|a| a.value.as_str())
}

/// Key/value pairs after `action_pos` until the next `action` or `_contract_address`.
fn wasm_kv_map_after_action(
    attrs: &[Attribute],
    action_pos: usize,
) -> std::collections::HashMap<&str, &str> {
    let mut m = std::collections::HashMap::new();
    let mut i = action_pos.saturating_add(1);
    while i < attrs.len() {
        let k = attrs[i].key.as_str();
        if k == "action" || k == "_contract_address" {
            break;
        }
        m.insert(k, attrs[i].value.as_str());
        i += 1;
    }
    m
}

fn parse_positive_raw(s: &str) -> Option<BigDecimal> {
    let n: BigDecimal = s.parse().ok()?;
    if n > BigDecimal::from(0) {
        Some(n)
    } else {
        None
    }
}

/// Token identity for a wrap-mapper fee segment. Fail closed if missing.
///
/// Prefers explicit fee token attrs, then native denom. Never uses `symbol=` (A1 spoof).
fn wrap_fee_token_from_seg(seg: &std::collections::HashMap<&str, &str>) -> Option<String> {
    for key in ["fee_asset", "fee_denom", "native_denom", "denom"] {
        if let Some(v) = seg.get(key).copied() {
            let t = v.trim();
            if !t.is_empty() && !t.chars().any(char::is_whitespace) {
                return Some(t.to_string());
            }
        }
    }
    None
}

/// Captured ustr-cmm amount key is `fee`. `fee_amount` is a #586 alias only.
fn wrap_fee_amount_from_seg(seg: &std::collections::HashMap<&str, &str>) -> Option<BigDecimal> {
    for key in ["fee", "fee_amount"] {
        if let Some(v) = seg.get(key).copied() {
            if let Some(n) = parse_positive_raw(v) {
                return Some(n);
            }
        }
    }
    None
}

fn wrap_fee_source(action: &str) -> Option<FeeSource> {
    match action {
        // Captured wrap-mapper wrap execute (treasury `wrap_deposit` is not a fee).
        "notify_deposit" | "wrap" => Some(FeeSource::Wrap),
        "unwrap" => Some(FeeSource::Unwrap),
        _ => None,
    }
}

fn parse_wrap_fees_from_attrs(
    attrs: &[Attribute],
    pinned: &str,
    ordinal: &mut i64,
) -> Vec<ParsedWrapFee> {
    let mut out = Vec::new();
    for (i, a) in attrs.iter().enumerate() {
        if a.key != "action" {
            continue;
        }
        let Some(source) = wrap_fee_source(a.value.as_str()) else {
            continue;
        };
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            continue;
        };
        if !wrap_mapper_matches(pinned, contract) {
            continue;
        }
        let seg = wasm_kv_map_after_action(attrs, i);
        // Fail closed: require an explicit treasury fee amount. Do not use `amount × bps`.
        // Do not count burn_tax / tax_amount / InstantWithdraw remainder / wrap_deposit amount.
        let Some(amount_raw) = wrap_fee_amount_from_seg(&seg) else {
            continue;
        };
        let Some(token) = wrap_fee_token_from_seg(&seg) else {
            continue;
        };
        out.push(ParsedWrapFee {
            source,
            amount_raw,
            token,
            ordinal: *ordinal,
        });
        *ordinal += 1;
    }
    out
}

/// Extract wrap/unwrap treasury fees from LCD wasm events.
///
/// **Locked from ustr-cmm wrap-mapper** (`contracts/wrap-mapper/src/contract.rs`):
/// `_contract_address`, `action` = `notify_deposit` (wrap) \| `unwrap`, amount key
/// **`fee`**, token `denom`. Legacy `action=wrap` + `fee_amount` still accepted.
/// Flattened combo txs are scanned per `action` (#141 / #285). Burn-tax attrs ignored.
pub fn parse_wrap_fees(tx: &TxResponse, pinned_mapper: &str) -> Vec<ParsedWrapFee> {
    if pinned_mapper.is_empty() {
        return Vec::new();
    }
    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    let mut out = Vec::new();
    let mut ordinal = 0i64;
    for event in events {
        if event.event_type != "wasm" && event.event_type != "wasm-wasm" {
            continue;
        }
        out.extend(parse_wrap_fees_from_attrs(
            &event.attributes,
            pinned_mapper,
            &mut ordinal,
        ));
    }
    out
}

/// Token identity for a ust1-window fee. Fail closed if missing.
///
/// Same allowlist as wrap. Never `symbol=` (A1 spoof). Never vFDUSD CEX ticker.
fn window_fee_token(seg: &std::collections::HashMap<&str, &str>) -> Option<String> {
    for key in [
        "fee_asset",
        "fee_denom",
        "native_denom",
        "denom",
        "ust1_token",
    ] {
        if let Some(v) = seg.get(key) {
            let t = v.trim();
            if !t.is_empty() && !t.chars().any(char::is_whitespace) {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn wasm_contract_addr_before_action<'a>(
    attrs: &'a [Attribute],
    action_idx: usize,
) -> Option<&'a str> {
    attrs[..action_idx]
        .iter()
        .rev()
        .find(|a| a.key == "_contract_address")
        .map(|a| a.value.as_str())
}

fn wasm_kv_after_action<'a>(
    attrs: &'a [Attribute],
    action_idx: usize,
) -> std::collections::HashMap<&'a str, &'a str> {
    let mut m = std::collections::HashMap::new();
    let mut i = action_idx.saturating_add(1);
    while i < attrs.len() {
        let k = attrs[i].key.as_str();
        if k == "action" || k == "_contract_address" {
            break;
        }
        m.insert(k, attrs[i].value.as_str());
        i += 1;
    }
    m
}

/// Extract UST1 window mint/redeem treasury fees from LCD wasm events (GitLab #614).
///
/// Locked crate attrs (ust1-window columbus-5 **11566** / `contracts/ust1-window`):
/// `action` = `deposit` \| `withdraw`, plus `ust1_out` / `vfdusd_out`,
/// `fee_total_bps`, `fee_chain_tax_bps`, `fee_cmm_protocol_bps`, and on deposit
/// `vfdusd_to_treasury`. **Those are not a treasury fee amount.**
///
/// Same address **11618** ([ust1-window#33](https://gitlab.com/PlasticDigits/ust1-window/-/issues/33))
/// emits explicit `fee_amount` + `fee_asset` (UST1). Ingest requires that
/// positive raw amount + token identity (`fee_asset` / `fee_denom` /
/// `native_denom` / `denom` / `ust1_token`) on the window `_contract_address`
/// segment. Do not infer `ust1_out × fee_total_bps`. Flattened CW20 `send` +
/// hook is scanned per `action` (#285 reserved key only). **I614-1–I614-8**.
pub fn parse_ust1_window_fees(tx: &TxResponse, pinned_window: &str) -> Vec<ParsedWrapFee> {
    if pinned_window.is_empty() {
        return Vec::new();
    }
    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    let mut out = Vec::new();
    let mut ordinal = 0i64;
    for event in events {
        if event.event_type != "wasm" && event.event_type != "wasm-wasm" {
            continue;
        }
        for (i, attr) in event.attributes.iter().enumerate() {
            if attr.key != "action" {
                continue;
            }
            let source = match attr.value.as_str() {
                "deposit" => FeeSource::Ust1Mint,
                "withdraw" => FeeSource::Ust1Redeem,
                _ => continue,
            };
            let Some(contract) = wasm_contract_addr_before_action(&event.attributes, i) else {
                continue;
            };
            if !pin_matches(pinned_window, contract) {
                continue;
            }
            let seg = wasm_kv_after_action(&event.attributes, i);
            // Fail closed: require explicit treasury fee amount. Never × fee_bps.
            // Do not treat ust1_out / vfdusd_out / vfdusd_to_treasury / min_vfdusd_out as the fee.
            let Some(amount_raw) = seg.get("fee_amount").copied().and_then(parse_positive_raw)
            else {
                continue;
            };
            let Some(token) = window_fee_token(&seg) else {
                continue;
            };
            out.push(ParsedWrapFee {
                source,
                amount_raw,
                token,
                ordinal,
            });
            ordinal += 1;
        }
    }
    out
}

/// Humanize raw fee × P522-Q/hub. Overflow / non-positive / unpriced → `None`.
pub fn fee_usd_for_raw(
    asset: &AssetRow,
    raw: &BigDecimal,
    ustc_usd: Option<&BigDecimal>,
    lunc_usd: Option<&BigDecimal>,
    configured_ustc_denom: Option<&str>,
    hub: Option<&HubQuoteUsd>,
) -> Option<BigDecimal> {
    if raw <= &BigDecimal::from(0) {
        return None;
    }
    let usd_per = catalog_usd_per_human_identity(
        &asset.symbol,
        asset.denom.as_deref(),
        asset.is_cw20,
        asset.contract_address.as_deref(),
        ustc_usd,
        lunc_usd,
        configured_ustc_denom,
        hub,
    )?;
    let human = humanize_raw_amount(raw, asset.decimals)?;
    let usd = human * usd_per;
    if usd <= BigDecimal::from(0) || !fits_numeric_38_18(&usd) {
        None
    } else {
        Some(usd)
    }
}

/// Flow Δ% = `(current − prior) / prior × 100` when `prior > 0`. Else `None` (never Inf).
pub fn flow_change_pct(
    current: Option<&BigDecimal>,
    prior: Option<&BigDecimal>,
) -> Option<BigDecimal> {
    let current = current?;
    let prior = prior?;
    if *prior <= BigDecimal::from(0) {
        return None;
    }
    let pct = (current - prior) / prior * BigDecimal::from(100);
    if !fits_numeric_38_18(&pct) {
        None
    } else {
        Some(pct)
    }
}

/// Overview JSON: `"0"` only when idle (no events); activity + unpriced → `null`.
pub fn overview_fee_usd_field(event_count: i64, usd: Option<&BigDecimal>) -> Option<String> {
    if event_count <= 0 {
        return Some("0".to_string());
    }
    match usd {
        Some(v) if *v > BigDecimal::from(0) => Some(v.to_string()),
        _ => None,
    }
}

/// Bid placement / fill commission is token1; ask is token0.
pub fn offer_asset_id_for_side(side: &str, asset_0_id: i32, asset_1_id: i32) -> Option<i32> {
    match side {
        "bid" => Some(asset_1_id),
        "ask" => Some(asset_0_id),
        _ => None,
    }
}

/// Row ready for `protocol_fee_events` insert.
#[derive(Debug, Clone)]
pub struct FeeEventDraft {
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub source: FeeSource,
    pub ordinal: i64,
    pub asset_id: i32,
    pub amount_raw: BigDecimal,
    pub decimals: i16,
    pub fee_usd: Option<BigDecimal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcd::{Attribute, Event, TxResponse};
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    fn attr(key: &str, value: &str) -> Attribute {
        Attribute {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    fn tx_with_wasm(attrs: Vec<Attribute>) -> TxResponse {
        TxResponse {
            height: "1".into(),
            txhash: "TX".into(),
            timestamp: None,
            logs: Some(vec![crate::lcd::TxLog {
                events: vec![Event {
                    event_type: "wasm".into(),
                    attributes: attrs,
                }],
            }]),
            events: None,
        }
    }

    #[test]
    fn wrap_mapper_pin_rejects_garbage() {
        assert!(parse_wrap_mapper_address("").is_none());
        assert!(parse_wrap_mapper_address("   ").is_none());
        assert!(parse_wrap_mapper_address("terra1abc\n").is_none());
        assert!(parse_wrap_mapper_address("terra1abc def").is_none());
        assert!(parse_wrap_mapper_address("osmo1aaaaaaaaaaaaaaaaaaa").is_none());
        assert!(parse_wrap_mapper_address("TERRA1aaaaaaaaaaaaaaaaaaa").is_none());
        let ok = parse_wrap_mapper_address(
            "  terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2  ",
        )
        .unwrap();
        assert!(ok.starts_with("terra1"));
        assert!(!ok.contains(' '));
    }

    #[test]
    fn wrap_mapper_match_is_exact_not_substring() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        assert!(wrap_mapper_matches(pin, pin));
        assert!(!wrap_mapper_matches(pin, "terra1spoof"));
        assert!(!wrap_mapper_matches(
            pin,
            "prefixterra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2"
        ));
        assert!(!wrap_mapper_matches("", pin));
    }

    #[test]
    fn parse_wrap_fees_pinned_only() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "wrap"),
            attr("fee_amount", "20000"),
            attr("denom", "uusd"),
        ]);
        let fees = parse_wrap_fees(&tx, pin);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Wrap);
        assert_eq!(fees[0].amount_raw, bd("20000"));
        assert_eq!(fees[0].token, "uusd");
    }

    #[test]
    fn parse_wrap_fees_spoof_contract_ignored() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr(
                "_contract_address",
                "terra1attackerxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("action", "wrap"),
            attr("fee_amount", "999999"),
            attr("denom", "uusd"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    #[test]
    fn parse_wrap_fees_missing_amount_fail_closed() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "unwrap"),
            attr("denom", "uusd"),
            attr("tax_amount", "15000"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    #[test]
    fn parse_unwrap_uses_fee_amount_not_tax_amount() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "unwrap"),
            attr("fee_amount", "1000"),
            attr("tax_amount", "50000"),
            attr("denom", "uluna"),
        ]);
        let fees = parse_wrap_fees(&tx, pin);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Unwrap);
        assert_eq!(fees[0].amount_raw, bd("1000"));
    }

    #[test]
    fn parse_wrap_ignores_instant_withdraw_burn_tax() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "instant_withdraw"),
            attr("tax_amount", "50000"),
            attr("fee_amount", "1"),
            attr("denom", "uluna"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    #[test]
    fn parse_wrap_fees_zero_amount_skipped() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "wrap"),
            attr("fee_amount", "0"),
            attr("denom", "uusd"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    /// Captured ustr-cmm wrap-mapper attrs (GitLab #613).
    /// Wrap execute is `notify_deposit` + `fee` + `denom` — not `action=wrap` / `fee_amount`.
    /// Source: PlasticDigits2/ustr-cmm `wrap-mapper` `execute_notify_deposit` /
    /// `execute_receive_cw20`. Columbus-5 mapper
    /// `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2`.
    fn captured_notify_deposit_attrs(pin: &str, denom: &str, fee: &str) -> Vec<Attribute> {
        vec![
            attr("_contract_address", pin),
            attr("action", "notify_deposit"),
            attr(
                "depositor",
                "terra1userxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("denom", denom),
            attr("gross_amount", "1000000"),
            attr("fee", fee),
            attr("fee_wrap_bps", "200"),
            attr("mint_amount", "980000"),
        ]
    }

    fn captured_unwrap_attrs(pin: &str, denom: &str, fee: &str) -> Vec<Attribute> {
        vec![
            attr("_contract_address", pin),
            attr("action", "unwrap"),
            attr(
                "cw20_contract",
                "terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch",
            ),
            attr(
                "recipient",
                "terra1userxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("denom", denom),
            attr("gross_amount", "1000000"),
            attr("fee", fee),
            attr("fee_unwrap_bps", "51"),
            attr("withdraw_amount", "994900"),
        ]
    }

    #[test]
    fn parse_wrap_captured_notify_deposit_uusd() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(captured_notify_deposit_attrs(pin, "uusd", "20000"));
        let fees = parse_wrap_fees(&tx, pin);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Wrap);
        assert_eq!(fees[0].amount_raw, bd("20000"));
        assert_eq!(fees[0].token, "uusd");
    }

    #[test]
    fn parse_wrap_captured_notify_deposit_uluna() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(captured_notify_deposit_attrs(pin, "uluna", "20000"));
        let fees = parse_wrap_fees(&tx, pin);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Wrap);
        assert_eq!(fees[0].token, "uluna");
    }

    #[test]
    fn parse_unwrap_captured_fee_not_tax_amount() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let mut attrs = captured_unwrap_attrs(pin, "uusd", "5100");
        attrs.push(attr("tax_amount", "15000"));
        let tx = tx_with_wasm(attrs);
        let fees = parse_wrap_fees(&tx, pin);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Unwrap);
        assert_eq!(fees[0].amount_raw, bd("5100"));
        assert_eq!(fees[0].token, "uusd");
    }

    #[test]
    fn parse_wrap_flattened_combo_keeps_wrap_when_last_action_is_swap() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let treasury = "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2";
        let pair = "terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f";
        let mut attrs = vec![
            attr("_contract_address", treasury),
            attr("action", "wrap_deposit"),
            attr(
                "depositor",
                "terra1userxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("denom", "uusd"),
            attr("amount", "1000000"),
        ];
        attrs.extend(captured_notify_deposit_attrs(pin, "uusd", "20000"));
        attrs.extend([
            attr("_contract_address", pair),
            attr("action", "swap"),
            attr("commission_amount", "3000"),
            attr("offer_amount", "980000"),
            attr("return_amount", "900000"),
        ]);
        let tx = tx_with_wasm(attrs);
        let fees = parse_wrap_fees(&tx, pin);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Wrap);
        assert_eq!(fees[0].amount_raw, bd("20000"));
        assert_eq!(fees[0].token, "uusd");
    }

    #[test]
    fn parse_unwrap_flattened_ignores_instant_withdraw_tax() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let treasury = "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2";
        let mut attrs = captured_unwrap_attrs(pin, "uluna", "5100");
        attrs.extend([
            attr("_contract_address", treasury),
            attr("action", "instant_withdraw"),
            attr(
                "recipient",
                "terra1userxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("denom", "uluna"),
            attr("amount", "994900"),
            attr("tax_amount", "14923"),
            attr("fee_amount", "999999"),
        ]);
        let tx = tx_with_wasm(attrs);
        let fees = parse_wrap_fees(&tx, pin);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Unwrap);
        assert_eq!(fees[0].amount_raw, bd("5100"));
        assert_eq!(fees[0].token, "uluna");
    }

    #[test]
    fn parse_wrap_deposit_without_mapper_fee_drops() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let treasury = "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2";
        let tx = tx_with_wasm(vec![
            attr("_contract_address", treasury),
            attr("action", "wrap_deposit"),
            attr("denom", "uusd"),
            attr("amount", "1000000"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    #[test]
    fn parse_wrap_forged_contract_address_ignored() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr("contract_address", pin),
            attr("action", "notify_deposit"),
            attr("fee", "20000"),
            attr("denom", "uusd"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    #[test]
    fn parse_wrap_missing_fee_and_token_fail_closed() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let no_fee = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "notify_deposit"),
            attr("denom", "uusd"),
            attr("gross_amount", "1000000"),
            attr("fee_wrap_bps", "200"),
        ]);
        assert!(parse_wrap_fees(&no_fee, pin).is_empty());
        let no_token = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "notify_deposit"),
            attr("fee", "20000"),
            attr("gross_amount", "1000000"),
        ]);
        assert!(parse_wrap_fees(&no_token, pin).is_empty());
    }

    #[test]
    fn parse_wrap_never_infers_amount_times_bps() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr("_contract_address", pin),
            attr("action", "notify_deposit"),
            attr("denom", "uusd"),
            attr("gross_amount", "1000000"),
            attr("fee_wrap_bps", "200"),
            attr("amount", "1000000"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    #[test]
    fn parse_wrap_spoof_notify_deposit_ignored() {
        let pin = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2";
        let tx = tx_with_wasm(vec![
            attr(
                "_contract_address",
                "terra1attackerxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("action", "notify_deposit"),
            attr("fee", "999999"),
            attr("denom", "uusd"),
        ]);
        assert!(parse_wrap_fees(&tx, pin).is_empty());
    }

    #[test]
    fn flow_change_pct_never_inf() {
        assert_eq!(
            flow_change_pct(Some(&bd("150")), Some(&bd("100"))).unwrap(),
            bd("50")
        );
        assert_eq!(
            flow_change_pct(Some(&bd("100")), Some(&bd("100"))).unwrap(),
            bd("0")
        );
        assert!(flow_change_pct(Some(&bd("150")), Some(&bd("0"))).is_none());
        assert!(flow_change_pct(Some(&bd("150")), None).is_none());
        assert!(flow_change_pct(None, Some(&bd("100"))).is_none());
        assert!(flow_change_pct(Some(&bd("0")), Some(&bd("100"))).unwrap() < bd("0"));
    }

    #[test]
    fn overview_fee_usd_idle_vs_unpriced() {
        assert_eq!(
            overview_fee_usd_field(0, Some(&bd("12"))),
            Some("0".to_string())
        );
        assert_eq!(overview_fee_usd_field(3, Some(&bd("0"))), None);
        assert_eq!(overview_fee_usd_field(3, None), None);
        assert_eq!(
            overview_fee_usd_field(2, Some(&bd("4.5"))),
            Some("4.5".to_string())
        );
    }

    #[test]
    fn offer_asset_side_mapping() {
        assert_eq!(offer_asset_id_for_side("bid", 10, 20), Some(20));
        assert_eq!(offer_asset_id_for_side("ask", 10, 20), Some(10));
        assert_eq!(offer_asset_id_for_side("sideways", 10, 20), None);
    }

    #[test]
    fn fee_source_unknown_rejected() {
        assert!(FeeSource::parse("javascript:").is_none());
        assert!(FeeSource::parse("swap").is_none());
        assert!(FeeSource::parse("deposit").is_none());
        assert!(FeeSource::parse("withdraw").is_none());
        assert_eq!(FeeSource::parse("swap_amm"), Some(FeeSource::SwapAmm));
        assert_eq!(FeeSource::parse("ust1_mint"), Some(FeeSource::Ust1Mint));
        assert_eq!(FeeSource::parse("ust1_redeem"), Some(FeeSource::Ust1Redeem));
        assert_eq!(fee_source_label(FeeSource::Ust1Mint), "UST1 mint");
        assert_eq!(fee_source_label(FeeSource::Ust1Redeem), "UST1 redeem");
    }

    /// Columbus-5 window (REGISTRY.md / ust1-window code 11566).
    const WINDOW: &str = "terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2";
    /// Hub UST1 CW20 — fee token identity when wasm emits it.
    const UST1: &str = "terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72";

    #[test]
    fn ust1_window_pin_rejects_garbage() {
        assert!(parse_ust1_window_address("").is_none());
        assert!(parse_ust1_window_address("   ").is_none());
        assert!(parse_ust1_window_address("terra1abc def").is_none());
        assert!(parse_ust1_window_address("osmo1aaaaaaaaaaaaaaaaaaa").is_none());
        assert_eq!(
            parse_ust1_window_address(&format!("  {WINDOW}  ")).as_deref(),
            Some(WINDOW)
        );
    }

    #[test]
    fn parse_ust1_window_deposit_fixture() {
        let tx = tx_with_wasm(vec![
            attr("_contract_address", WINDOW),
            attr("action", "deposit"),
            attr("ust1_out", "990000"),
            attr("fee_total_bps", "100"),
            attr("fee_chain_tax_bps", "50"),
            attr("fee_cmm_protocol_bps", "50"),
            attr("vfdusd_to_treasury", "1000000"),
            attr("fee_amount", "10000"),
            attr("fee_asset", UST1),
        ]);
        let fees = parse_ust1_window_fees(&tx, WINDOW);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Ust1Mint);
        assert_eq!(fees[0].amount_raw, bd("10000"));
        assert_eq!(fees[0].token, UST1);
    }

    #[test]
    fn parse_ust1_window_withdraw_fixture() {
        let tx = tx_with_wasm(vec![
            attr("_contract_address", WINDOW),
            attr("action", "withdraw"),
            attr("vfdusd_out", "980000"),
            attr("fee_total_bps", "100"),
            attr("min_vfdusd_out", "970000"),
            attr("fee_amount", "10000"),
            attr("fee_asset", UST1),
        ]);
        let fees = parse_ust1_window_fees(&tx, WINDOW);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Ust1Redeem);
        assert_eq!(fees[0].amount_raw, bd("10000"));
        assert_eq!(fees[0].token, UST1);
    }

    #[test]
    fn parse_ust1_window_crate_attrs_without_fee_amount_fail_closed() {
        // Locked 11566 crate attrs — no fee_amount. Do not infer from fee_total_bps.
        let tx = tx_with_wasm(vec![
            attr("_contract_address", WINDOW),
            attr("action", "deposit"),
            attr("ust1_out", "990000"),
            attr("fee_total_bps", "100"),
            attr("fee_chain_tax_bps", "50"),
            attr("fee_cmm_protocol_bps", "50"),
            attr("vfdusd_to_treasury", "1000000"),
        ]);
        assert!(parse_ust1_window_fees(&tx, WINDOW).is_empty());
    }

    #[test]
    fn parse_ust1_window_spoof_and_forged_contract_address_ignored() {
        let spoof = tx_with_wasm(vec![
            attr(
                "_contract_address",
                "terra1attackerxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("action", "deposit"),
            attr("fee_amount", "999999"),
            attr("fee_asset", UST1),
        ]);
        assert!(parse_ust1_window_fees(&spoof, WINDOW).is_empty());

        let forged = tx_with_wasm(vec![
            attr(
                "_contract_address",
                "terra1attackerxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            attr("contract_address", WINDOW),
            attr("action", "withdraw"),
            attr("fee_amount", "999999"),
            attr("fee_asset", UST1),
        ]);
        assert!(parse_ust1_window_fees(&forged, WINDOW).is_empty());
    }

    #[test]
    fn parse_ust1_window_flattened_send_plus_hook_scopes_window() {
        let tx = tx_with_wasm(vec![
            attr(
                "_contract_address",
                "terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3",
            ),
            attr("action", "send"),
            attr("amount", "1000000"),
            attr("_contract_address", WINDOW),
            attr("action", "deposit"),
            attr("ust1_out", "990000"),
            attr("fee_total_bps", "100"),
            attr("fee_amount", "10000"),
            attr("fee_asset", UST1),
            attr("_contract_address", UST1),
            attr("action", "mint"),
            attr("amount", "990000"),
        ]);
        let fees = parse_ust1_window_fees(&tx, WINDOW);
        assert_eq!(fees.len(), 1);
        assert_eq!(fees[0].source, FeeSource::Ust1Mint);
        assert_eq!(fees[0].amount_raw, bd("10000"));
        assert_eq!(fees[0].token, UST1);
    }

    #[test]
    fn parse_ust1_window_empty_pin_omits() {
        let tx = tx_with_wasm(vec![
            attr("_contract_address", WINDOW),
            attr("action", "deposit"),
            attr("fee_amount", "10000"),
            attr("fee_asset", UST1),
        ]);
        assert!(parse_ust1_window_fees(&tx, "").is_empty());
    }
}
