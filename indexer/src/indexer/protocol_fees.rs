//! Protocol treasury fee ingest + wrap-mapper pin (GitLab #586).
//!
//! **PFee / L7:** count amounts sent to pair / wrap-mapper treasury only.
//! Do not count `spread_amount`, Terra Classic burn tax, gas, `hook_fee_amount`,
//! LP, book escrow, or parked dust. Hybrid book taker is `limit_order_fills.commission_amount`
//! — never also add a swap-level `book_commission_amount`.
//!
//! Wrap/unwrap wasm is accepted only from a pinned `WRAP_MAPPER_ADDRESS` (exact bech32).
//! Fail closed on missing `fee_amount` / token identity. Do not infer `amount × bps`.

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
}

impl FeeSource {
    pub const ALL: [FeeSource; 5] = [
        FeeSource::SwapAmm,
        FeeSource::BookTake,
        FeeSource::LimitPlace,
        FeeSource::Wrap,
        FeeSource::Unwrap,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FeeSource::SwapAmm => "swap_amm",
            FeeSource::BookTake => "book_take",
            FeeSource::LimitPlace => "limit_place",
            FeeSource::Wrap => "wrap",
            FeeSource::Unwrap => "unwrap",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "swap_amm" => Some(FeeSource::SwapAmm),
            "book_take" => Some(FeeSource::BookTake),
            "limit_place" => Some(FeeSource::LimitPlace),
            "wrap" => Some(FeeSource::Wrap),
            "unwrap" => Some(FeeSource::Unwrap),
            _ => None,
        }
    }

    pub fn is_wrap_family(self) -> bool {
        matches!(self, FeeSource::Wrap | FeeSource::Unwrap)
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

/// Pin `WRAP_MAPPER_ADDRESS`: trim, reject whitespace/newline/wrong HRP, no `LIKE %`.
///
/// Accepts `terra1` bech32 only (columbus-5 / LocalTerra). Empty / garbage → `None`
/// (wrap source omitted, not fake idle `$0`).
pub fn parse_wrap_mapper_address(raw: &str) -> Option<String> {
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

/// Exact bech32 equality — never substring / case-fold.
pub fn wrap_mapper_matches(pinned: &str, contract: &str) -> bool {
    !pinned.is_empty() && pinned == contract
}

fn wasm_attr_last<'a>(attributes: &'a [Attribute], key: &str) -> Option<&'a str> {
    attributes
        .iter()
        .rev()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
}

fn wasm_contract_addr(attributes: &[Attribute]) -> Option<&str> {
    wasm_attr_last(attributes, "_contract_address")
}

fn parse_positive_raw(s: &str) -> Option<BigDecimal> {
    let n: BigDecimal = s.parse().ok()?;
    if n > BigDecimal::from(0) {
        Some(n)
    } else {
        None
    }
}

/// Token identity for a wrap-mapper fee. Fail closed if missing.
///
/// Prefers explicit fee token attrs, then native denom. Never uses `symbol=` (A1 spoof).
fn wrap_fee_token(attrs: &[Attribute]) -> Option<String> {
    for key in ["fee_asset", "fee_denom", "native_denom", "denom"] {
        if let Some(v) = wasm_attr_last(attrs, key) {
            let t = v.trim();
            if !t.is_empty() && !t.chars().any(char::is_whitespace) {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn parse_wrap_fee_from_attrs(attrs: &[Attribute], pinned: &str, ordinal: i64) -> Option<ParsedWrapFee> {
    let contract = wasm_contract_addr(attrs)?;
    if !wrap_mapper_matches(pinned, contract) {
        return None;
    }
    let action = wasm_attr_last(attrs, "action")?;
    let source = match action {
        "wrap" => FeeSource::Wrap,
        "unwrap" => FeeSource::Unwrap,
        _ => return None,
    };
    // Fail closed: require an explicit treasury fee amount. Do not use `amount × bps`.
    // Do not count burn_tax / tax_amount / InstantWithdraw remainder.
    let amount_raw = wasm_attr_last(attrs, "fee_amount").and_then(parse_positive_raw)?;
    let token = wrap_fee_token(attrs)?;
    Some(ParsedWrapFee {
        source,
        amount_raw,
        token,
        ordinal,
    })
}

/// Extract wrap/unwrap treasury fees from LCD wasm events.
///
/// Documented ustr-cmm (post-#9) attrs: `_contract_address`, `action` = `wrap` \| `unwrap`,
/// `fee_amount` (raw integer to treasury), token via `fee_asset` / `fee_denom` /
/// `native_denom` / `denom`. Burn-tax attrs are ignored.
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
        if let Some(fee) = parse_wrap_fee_from_attrs(&event.attributes, pinned_mapper, ordinal) {
            ordinal += 1;
            out.push(fee);
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
pub fn flow_change_pct(current: Option<&BigDecimal>, prior: Option<&BigDecimal>) -> Option<BigDecimal> {
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
            attr("_contract_address", "terra1attackerxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
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
        assert_eq!(FeeSource::parse("swap_amm"), Some(FeeSource::SwapAmm));
    }
}
