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
}
