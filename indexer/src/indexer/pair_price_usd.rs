//! USD of 1 human unit of pair **base** (`asset_0`) at index time (GitLab #522).
//!
//! `price_usd = human_quote_per_base * usd_per_human_quote` using the #515 ticker
//! oracles plus a small quote-token catalog (P522-Q):
//!
//! | Quote (symbol / denom) | USD handle |
//! |------------------------|------------|
//! | UST1 | `$1` |
//! | USTC, cUSTC / CUSTC, `uusd` | USTC oracle |
//! | LUNC, cLUNC / CLUNC, `uluna` | LUNC oracle |
//! | USTR | `2.5 ×` USTC oracle (seed peg, #508 / #522) |
//!
//! Unknown quotes → `None` (do not invent a USD). Advisory only — not settlement.

use std::str::FromStr;

use bigdecimal::BigDecimal;

use crate::db::queries::assets::AssetRow;

/// USTR human units per 1 USTC human unit used for the #508 seed peg (P522-Q).
pub const USTR_PER_USTC: &str = "2.5";

/// `10^exp` as [`BigDecimal`] (`exp` may be negative).
pub fn ten_pow_i32(exp: i32) -> BigDecimal {
    if exp == 0 {
        return BigDecimal::from(1);
    }
    let mag = exp.unsigned_abs();
    let ten = BigDecimal::from(10);
    let mut v = BigDecimal::from(1);
    for _ in 0..mag {
        v *= &ten;
    }
    if exp < 0 {
        BigDecimal::from(1) / v
    } else {
        v
    }
}

/// Factor that converts a **raw** integer quote-per-base ratio into **human** quote-per-base.
///
/// `human = raw * 10^(decimals_base − decimals_quote)` (P522-1).
pub fn human_price_scale(decimals_base: i16, decimals_quote: i16) -> BigDecimal {
    ten_pow_i32(i32::from(decimals_base) - i32::from(decimals_quote))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteUsdKind {
    Peg1,
    Ustc,
    Lunc,
    Ustr,
}

/// Classify a quote (or base) asset for USD conversion.
pub fn quote_usd_kind(symbol: &str, denom: Option<&str>) -> Option<QuoteUsdKind> {
    if let Some(d) = denom {
        match d {
            "uusd" => return Some(QuoteUsdKind::Ustc),
            "uluna" => return Some(QuoteUsdKind::Lunc),
            _ => {}
        }
    }
    match normalize_symbol(symbol).as_str() {
        "UST1" => Some(QuoteUsdKind::Peg1),
        "USTC" | "CUSTC" => Some(QuoteUsdKind::Ustc),
        "LUNC" | "CLUNC" => Some(QuoteUsdKind::Lunc),
        "USTR" => Some(QuoteUsdKind::Ustr),
        _ => None,
    }
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

/// USD per 1 human unit of the quote asset.
pub fn usd_per_human_quote(
    kind: QuoteUsdKind,
    ustc_usd: Option<&BigDecimal>,
    lunc_usd: Option<&BigDecimal>,
) -> Option<BigDecimal> {
    match kind {
        QuoteUsdKind::Peg1 => Some(BigDecimal::from(1)),
        QuoteUsdKind::Ustc => ustc_usd.cloned(),
        QuoteUsdKind::Lunc => lunc_usd.cloned(),
        QuoteUsdKind::Ustr => {
            let ustc = ustc_usd?;
            Some(ustc * BigDecimal::from_str(USTR_PER_USTC).ok()?)
        }
    }
}

/// USD of 1 human unit of pair base = human quote-per-base × quote USD.
pub fn usd_of_one_human_base(
    human_quote_per_base: &BigDecimal,
    quote_usd: &BigDecimal,
) -> BigDecimal {
    human_quote_per_base * quote_usd
}

/// Resolve `price_usd` for an oriented (human) quote-per-base print.
pub fn price_usd_for_human_quote_per_base(
    quote: &AssetRow,
    human_quote_per_base: &BigDecimal,
    ustc_usd: Option<&BigDecimal>,
    lunc_usd: Option<&BigDecimal>,
) -> Option<BigDecimal> {
    let kind = quote_usd_kind(&quote.symbol, quote.denom.as_deref())?;
    let quote_usd = usd_per_human_quote(kind, ustc_usd, lunc_usd)?;
    if quote_usd <= BigDecimal::from(0) {
        return None;
    }
    Some(usd_of_one_human_base(human_quote_per_base, &quote_usd))
}

/// Classify a factory asset row for USD notional (GitLab #548 / #544, **A1**).
///
/// Natives: only `uusd` / `uluna` (or `configured_ustc_denom`). A gem native that spoofs
/// `symbol=USTR` / `UST1` must **not** price. CW20 hub tickers require a contract address.
pub fn quote_usd_kind_for_asset(
    asset: &AssetRow,
    configured_ustc_denom: Option<&str>,
) -> Option<QuoteUsdKind> {
    if let Some(d) = asset.denom.as_deref() {
        if d == "uusd" {
            return Some(QuoteUsdKind::Ustc);
        }
        if d == "uluna" {
            return Some(QuoteUsdKind::Lunc);
        }
        if let Some(cfg) = configured_ustc_denom {
            if d == cfg {
                return Some(QuoteUsdKind::Ustc);
            }
        }
        if !asset.is_cw20 {
            return None;
        }
    }
    if let Some(cfg) = configured_ustc_denom {
        if let Some(addr) = asset.contract_address.as_deref() {
            if addr == cfg {
                return Some(QuoteUsdKind::Ustc);
            }
        }
    }
    if asset.is_cw20 && asset.contract_address.as_deref().unwrap_or("").is_empty() {
        return None;
    }
    quote_usd_kind(&asset.symbol, asset.denom.as_deref())
}

/// Raw integer amount → human units using the asset's decimals. `None` if non-positive.
pub fn humanize_raw_amount(raw: &BigDecimal, decimals: i16) -> Option<BigDecimal> {
    if raw <= &BigDecimal::from(0) {
        return None;
    }
    if decimals < 0 || decimals > 38 {
        return None;
    }
    let human = raw / ten_pow_i32(i32::from(decimals));
    if human <= BigDecimal::from(0) {
        None
    } else {
        Some(human)
    }
}

fn catalog_usd_per_human(
    asset: &AssetRow,
    ustc_usd: Option<&BigDecimal>,
    lunc_usd: Option<&BigDecimal>,
    configured_ustc_denom: Option<&str>,
) -> Option<BigDecimal> {
    let kind = quote_usd_kind_for_asset(asset, configured_ustc_denom)?;
    let usd = usd_per_human_quote(kind, ustc_usd, lunc_usd)?;
    if usd <= BigDecimal::from(0) {
        None
    } else {
        Some(usd)
    }
}

fn notional_usd(
    asset: &AssetRow,
    raw: &BigDecimal,
    usd_per_human: &BigDecimal,
) -> Option<BigDecimal> {
    let human = humanize_raw_amount(raw, asset.decimals)?;
    let usd = human * usd_per_human;
    if usd <= BigDecimal::from(0) {
        None
    } else {
        Some(usd)
    }
}

/// One USD notional per swap (**L10**): catalog-known **pair quote** (`asset_1`) preferred;
/// else offer; else ask. Never sum both legs. Unknown / missing oracle → `None` (not `$0`).
///
/// If the pair quote is in the catalog but the needed oracle is down, return `None`
/// rather than falling through to the other leg.
///
/// Shared ingest helper for Charts overview (#548) and pair-list USD (#544).
pub fn volume_usd_for_swap(
    offer: &AssetRow,
    ask: &AssetRow,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
    pair_quote: &AssetRow,
    ustc_usd: Option<&BigDecimal>,
    lunc_usd: Option<&BigDecimal>,
    configured_ustc_denom: Option<&str>,
) -> Option<BigDecimal> {
    if quote_usd_kind_for_asset(pair_quote, configured_ustc_denom).is_some() {
        let usd = catalog_usd_per_human(pair_quote, ustc_usd, lunc_usd, configured_ustc_denom)?;
        if offer.id == pair_quote.id {
            return notional_usd(offer, offer_amount, &usd);
        }
        if ask.id == pair_quote.id {
            return notional_usd(ask, return_amount, &usd);
        }
        return None;
    }
    if let Some(usd) = catalog_usd_per_human(offer, ustc_usd, lunc_usd, configured_ustc_denom) {
        return notional_usd(offer, offer_amount, &usd);
    }
    if let Some(usd) = catalog_usd_per_human(ask, ustc_usd, lunc_usd, configured_ustc_denom) {
        return notional_usd(ask, return_amount, &usd);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    fn asset(symbol: &str, denom: Option<&str>) -> AssetRow {
        AssetRow {
            id: 1,
            contract_address: None,
            denom: denom.map(str::to_string),
            is_cw20: denom.is_none(),
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            decimals: 6,
            logo_url: None,
            coingecko_id: None,
            cmc_id: None,
            first_seen_block: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn catalog_maps_wraps_and_denoms() {
        assert_eq!(quote_usd_kind("cUSTC", None), Some(QuoteUsdKind::Ustc));
        assert_eq!(quote_usd_kind("USTC", Some("uusd")), Some(QuoteUsdKind::Ustc));
        assert_eq!(quote_usd_kind("cLUNC", None), Some(QuoteUsdKind::Lunc));
        assert_eq!(quote_usd_kind("LUNC", Some("uluna")), Some(QuoteUsdKind::Lunc));
        assert_eq!(quote_usd_kind("UST1", None), Some(QuoteUsdKind::Peg1));
        assert_eq!(quote_usd_kind("USTR", None), Some(QuoteUsdKind::Ustr));
        assert_eq!(quote_usd_kind("CL8Y", None), None);
    }

    #[test]
    fn ust1_custc_last_print_is_about_one_dollar() {
        let quote = asset("cUSTC", None);
        let human = bd("206.62");
        let ustc = bd("0.004928");
        let usd = price_usd_for_human_quote_per_base(&quote, &human, Some(&ustc), None).unwrap();
        let f = {
            use bigdecimal::ToPrimitive;
            usd.to_f64().unwrap()
        };
        assert!((f - 1.018).abs() < 0.02, "got {f}");
    }

    #[test]
    fn ust1_ustr_last_print_is_about_one_dollar() {
        let quote = asset("USTR", None);
        let human = bd("79.72");
        let ustc = bd("0.004928");
        let usd = price_usd_for_human_quote_per_base(&quote, &human, Some(&ustc), None).unwrap();
        let f = {
            use bigdecimal::ToPrimitive;
            usd.to_f64().unwrap()
        };
        // 79.72 * 2.5 * 0.004928 ≈ 0.983
        assert!((f - 0.983).abs() < 0.02, "got {f}");
    }

    #[test]
    fn unknown_quote_yields_none() {
        let quote = asset("CL8Y", None);
        assert!(price_usd_for_human_quote_per_base(&quote, &bd("1"), Some(&bd("0.005")), None).is_none());
    }

    fn cw20(id: i32, symbol: &str, decimals: i16, contract: &str) -> AssetRow {
        AssetRow {
            id,
            contract_address: Some(contract.to_string()),
            denom: None,
            is_cw20: true,
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            decimals,
            logo_url: None,
            coingecko_id: None,
            cmc_id: None,
            first_seen_block: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn native(id: i32, symbol: &str, denom: &str, decimals: i16) -> AssetRow {
        AssetRow {
            id,
            contract_address: None,
            denom: Some(denom.to_string()),
            is_cw20: false,
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            decimals,
            logo_url: None,
            coingecko_id: None,
            cmc_id: None,
            first_seen_block: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn usd_f(v: &BigDecimal) -> f64 {
        use bigdecimal::ToPrimitive;
        v.to_f64().unwrap()
    }

    /// I1: 10 human USTR offered into UST1/USTR → 10 × 2.5 × USTC.
    #[test]
    fn volume_ust1_ustr_uses_human_ustr_times_peg() {
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ustr = cw20(2, "USTR", 18, "terra1ustr");
        let offer = bd("10000000000000000000"); // 10 human USTR
        let ret = bd("1000000"); // 1 human UST1 (unused when quote=USTR)
        let ustc = bd("0.004878");
        let usd = volume_usd_for_swap(
            &ustr, &ust1, &offer, &ret, &ustr, Some(&ustc), None, None,
        )
        .unwrap();
        let expected = 10.0 * 2.5 * 0.004878;
        assert!((usd_f(&usd) - expected).abs() < 1e-9, "got {}", usd_f(&usd));
    }

    /// I2: UST1/cUSTC — quote preferred, not sum of both legs.
    #[test]
    fn volume_ust1_custc_one_side_not_both() {
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let custc = cw20(2, "cUSTC", 6, "terra1custc");
        let offer = bd("1000000"); // 1 UST1
        let ret = bd("200000000"); // 200 cUSTC
        let ustc = bd("0.005");
        let usd = volume_usd_for_swap(
            &ust1, &custc, &offer, &ret, &custc, Some(&ustc), None, None,
        )
        .unwrap();
        // Quote = cUSTC → 200 × 0.005 = 1.0, not 1.0 + 200×0.005
        assert!((usd_f(&usd) - 1.0).abs() < 1e-9, "got {}", usd_f(&usd));
        let both = 1.0 + 200.0 * 0.005;
        assert!((usd_f(&usd) - both).abs() > 0.5);
    }

    /// I3: native USTC leg still prices (legacy).
    #[test]
    fn volume_native_ustc_leg_still_prices() {
        let lunc = native(1, "LUNC", "uluna", 6);
        let uusd = native(2, "USTC", "uusd", 6);
        let offer = bd("1000000");
        let ret = bd("200000000");
        let ustc = bd("0.005");
        let usd = volume_usd_for_swap(
            &lunc, &uusd, &offer, &ret, &uusd, Some(&ustc), None, None,
        )
        .unwrap();
        assert!((usd_f(&usd) - 1.0).abs() < 1e-9, "got {}", usd_f(&usd));
    }

    /// I4: LUNC-quoted swap uses LUNC oracle, not USTC.
    #[test]
    fn volume_clunc_uses_lunc_oracle() {
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let clunc = cw20(2, "cLUNC", 6, "terra1clunc");
        let offer = bd("1000000");
        let ret = bd("2000000"); // 2 human cLUNC
        let ustc = bd("0.005");
        let lunc = bd("0.00005");
        let usd = volume_usd_for_swap(
            &ust1, &clunc, &offer, &ret, &clunc, Some(&ustc), Some(&lunc), None,
        )
        .unwrap();
        assert!((usd_f(&usd) - 0.0001).abs() < 1e-12, "got {}", usd_f(&usd));
    }

    /// I5: unknown gem quote with no catalog base → None.
    #[test]
    fn volume_unknown_gem_is_none() {
        let gem = cw20(1, "GEMX", 6, "terra1gem");
        let other = cw20(2, "GEMY", 6, "terra1gemy");
        let ustc = bd("0.005");
        assert!(volume_usd_for_swap(
            &gem,
            &other,
            &bd("1000000"),
            &bd("1000000"),
            &other,
            Some(&ustc),
            None,
            None,
        )
        .is_none());
    }

    /// A1: native gem spoofing USTR symbol must not price as hub.
    #[test]
    fn volume_symbol_spoof_native_ustr_is_none() {
        let gem = cw20(1, "GEMX", 6, "terra1gem");
        let spoof = native(2, "USTR", "ugem", 18);
        let ustc = bd("0.005");
        assert!(quote_usd_kind_for_asset(&spoof, None).is_none());
        assert!(volume_usd_for_swap(
            &spoof,
            &gem,
            &bd("10000000000000000000"),
            &bd("1000000"),
            &spoof,
            Some(&ustc),
            None,
            None,
        )
        .is_none());
    }

    #[test]
    fn volume_missing_ustc_oracle_is_none_except_ust1_peg() {
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ustr = cw20(2, "USTR", 18, "terra1ustr");
        assert!(volume_usd_for_swap(
            &ustr,
            &ust1,
            &bd("10000000000000000000"),
            &bd("1000000"),
            &ustr,
            None,
            None,
            None,
        )
        .is_none());
        let gem = cw20(3, "GEM", 6, "terra1gem");
        let usd = volume_usd_for_swap(
            &ust1,
            &gem,
            &bd("2000000"),
            &bd("1"),
            &gem,
            None,
            None,
            None,
        )
        .unwrap();
        assert!((usd_f(&usd) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn volume_negative_or_zero_raw_is_none() {
        let ust1 = cw20(1, "UST1", 6, "terra1ust1");
        let ustr = cw20(2, "USTR", 18, "terra1ustr");
        let ustc = bd("0.005");
        assert!(volume_usd_for_swap(
            &ustr, &ust1, &bd("0"), &bd("1"), &ustr, Some(&ustc), None, None,
        )
        .is_none());
        assert!(volume_usd_for_swap(
            &ustr, &ust1, &bd("-1"), &bd("1"), &ustr, Some(&ustc), None, None,
        )
        .is_none());
    }
}
