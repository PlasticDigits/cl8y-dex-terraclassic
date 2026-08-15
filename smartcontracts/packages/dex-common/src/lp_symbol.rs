//! LP CW20 ticker derivation for pair instantiate (GitLab #518).
//!
//! Asset-symbol prefixes keep **ASCII alphanumeric** (`0-9`, `A-Z`, `a-z`) and
//! drop every other character. The join is `{a}-{b}-LP`.
//!
//! Workspace / upgraded `cw20-mintable` validates **`[a-zA-Z0-9\-]{3,12}`**.
//! columbus-5 classic Terraswap LP code (`[a-zA-Z\-]{3,12}`, no digits) **cannot**
//! instantiate `UST1-CUST-LP` / `CL8Y-CLUN-LP`. New pairs need factory
//! `lp_token_code_id` pointed at digit-allowing CW20 (upgrade script).
//!
//! LP **name** and CosmWasm **label** keep the unsanitized factory symbols.

/// Mintable / upgraded LP ticker length floor (same as classic).
pub const CW20_LP_SYMBOL_MIN_LEN: usize = 3;
/// Mintable / upgraded LP ticker length ceiling (`{4}-{4}-LP` == 12).
pub const CW20_LP_SYMBOL_MAX_LEN: usize = 12;

/// Alphanumeric characters taken from each asset symbol before joining.
pub const LP_SYMBOL_PREFIX_CHARS: usize = 4;

/// Fallback when prefixes collapse below the length floor. Digits allowed.
pub const FALLBACK_LP_TOKEN_SYMBOL: &str = "CL8Y-LP";

/// True when `symbol` matches **`[a-zA-Z0-9\-]{3,12}`** (byte length).
pub fn is_mintable_cw20_lp_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < CW20_LP_SYMBOL_MIN_LEN || bytes.len() > CW20_LP_SYMBOL_MAX_LEN {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
}

/// Classic Terraswap / columbus-5 LP ticker: **`[a-zA-Z\-]{3,12}`** (no digits).
pub fn is_classic_cw20_lp_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < CW20_LP_SYMBOL_MIN_LEN || bytes.len() > CW20_LP_SYMBOL_MAX_LEN {
        return false;
    }
    bytes.iter().all(|b| b.is_ascii_alphabetic() || *b == b'-')
}

/// ASCII alphanumeric only, up to `n` characters (all other chars dropped).
pub fn alphanumeric_prefix(symbol: &str, n: usize) -> String {
    symbol
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(n)
        .collect()
}

/// Collapse consecutive hyphens and trim leading/trailing hyphens.
pub fn collapse_hyphens(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_hyphen = false;
    for c in raw.chars() {
        if c == '-' {
            if !prev_hyphen && !out.is_empty() {
                out.push('-');
                prev_hyphen = true;
            }
        } else {
            out.push(c);
            prev_hyphen = false;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

/// LP ticker that always passes [`is_mintable_cw20_lp_symbol`].
///
/// `None` (or prefixes that collapse to an invalid ticker) → [`FALLBACK_LP_TOKEN_SYMBOL`].
pub fn derive_lp_token_symbol(token_symbols: Option<[&str; 2]>) -> String {
    match token_symbols {
        Some([a, b]) => {
            let short_a = alphanumeric_prefix(a, LP_SYMBOL_PREFIX_CHARS);
            let short_b = alphanumeric_prefix(b, LP_SYMBOL_PREFIX_CHARS);
            let derived = format!("{}-{}-LP", short_a, short_b);
            let collapsed = collapse_hyphens(&derived);
            if is_mintable_cw20_lp_symbol(&collapsed) {
                collapsed
            } else {
                FALLBACK_LP_TOKEN_SYMBOL.to_string()
            }
        }
        None => FALLBACK_LP_TOKEN_SYMBOL.to_string(),
    }
}

/// `(name, symbol, label)` for the LP CW20 instantiate + wasm label.
pub fn lp_token_instantiate_meta(token_symbols: Option<&[String; 2]>) -> (String, String, String) {
    match token_symbols {
        Some([a, b]) => (
            format!("{}-{} CL8YDEX LP", a, b),
            derive_lp_token_symbol(Some([a.as_str(), b.as_str()])),
            format!("{}-{} cl8ydex lp", a, b),
        ),
        None => (
            "CL8Y DEX LP Token".to_string(),
            FALLBACK_LP_TOKEN_SYMBOL.to_string(),
            "CL8Y DEX LP Token".to_string(),
        ),
    }
}

/// Pre-sanitize join (`take(4)` including punctuation). Used only in tests / docs.
pub fn legacy_unsanitized_lp_symbol(a: &str, b: &str) -> String {
    let short_a: String = a.chars().take(LP_SYMBOL_PREFIX_CHARS).collect();
    let short_b: String = b.chars().take(LP_SYMBOL_PREFIX_CHARS).collect();
    format!("{}-{}-LP", short_a, short_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mintable_keeps_digits_classic_rejects_them() {
        assert!(is_mintable_cw20_lp_symbol("UST1-CUST-LP"));
        assert!(is_mintable_cw20_lp_symbol("CL8Y-CLUN-LP"));
        assert!(is_mintable_cw20_lp_symbol("CL8Y-LP"));
        assert!(!is_classic_cw20_lp_symbol("UST1-CUST-LP"));
        assert!(!is_classic_cw20_lp_symbol("CL8Y-CLUN-LP"));
        assert!(!is_classic_cw20_lp_symbol("CL8Y-LP"));
        assert!(!is_mintable_cw20_lp_symbol("AB"));
        assert!(!is_mintable_cw20_lp_symbol("ABCDEFGHIJKLM"));
        assert!(!is_mintable_cw20_lp_symbol("UST_CUST-LP"));
    }

    #[test]
    fn sanitizes_non_alnum_keeps_digits() {
        assert_eq!(
            derive_lp_token_symbol(Some(["UST1", "cUSTC"])),
            "UST1-cUST-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["UST1", "CUSTC"])),
            "UST1-CUST-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["CL8Y", "cLUNC"])),
            "CL8Y-cLUN-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["CL8Y", "CLUNC"])),
            "CL8Y-CLUN-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["UST1", "USTR"])),
            "UST1-USTR-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["FOO_BAR", "BAZ!"])),
            "FOOB-BAZ-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["US-T1", "cUSTC"])),
            "UST1-cUST-LP"
        );
        assert!(is_mintable_cw20_lp_symbol(&derive_lp_token_symbol(Some([
            "UST1", "CUSTC"
        ]))));
        assert!(!is_classic_cw20_lp_symbol(&derive_lp_token_symbol(Some([
            "UST1", "CUSTC"
        ]))));
    }

    #[test]
    fn letter_only_symbols_unchanged() {
        assert_eq!(
            derive_lp_token_symbol(Some(["TKNA", "TKNB"])),
            "TKNA-TKNB-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["CLUNC", "CUSTC"])),
            "CLUN-CUST-LP"
        );
        assert!(is_classic_cw20_lp_symbol("CLUN-CUST-LP"));
    }

    #[test]
    fn fallback_when_missing_or_empty_after_sanitize() {
        assert_eq!(derive_lp_token_symbol(None), FALLBACK_LP_TOKEN_SYMBOL);
        assert_eq!(
            derive_lp_token_symbol(Some(["@@@", "!!!"])),
            FALLBACK_LP_TOKEN_SYMBOL
        );
        assert_eq!(FALLBACK_LP_TOKEN_SYMBOL, "CL8Y-LP");
        assert!(is_mintable_cw20_lp_symbol(FALLBACK_LP_TOKEN_SYMBOL));
        assert!(!is_classic_cw20_lp_symbol(FALLBACK_LP_TOKEN_SYMBOL));
    }

    #[test]
    fn instantiate_meta_keeps_full_symbols_on_name_and_label() {
        let (name, symbol, label) =
            lp_token_instantiate_meta(Some(&["UST1".into(), "CUSTC".into()]));
        assert_eq!(name, "UST1-CUSTC CL8YDEX LP");
        assert_eq!(symbol, "UST1-CUST-LP");
        assert_eq!(label, "UST1-CUSTC cl8ydex lp");
        let (name, symbol, label) = lp_token_instantiate_meta(None);
        assert_eq!(name, "CL8Y DEX LP Token");
        assert_eq!(symbol, FALLBACK_LP_TOKEN_SYMBOL);
        assert_eq!(label, "CL8Y DEX LP Token");
    }

    #[test]
    fn collapse_hyphens_trims_emptied_prefixes() {
        assert_eq!(collapse_hyphens("-LP"), "LP");
        assert_eq!(collapse_hyphens("--LP"), "LP");
        assert_eq!(collapse_hyphens("A--B-LP"), "A-B-LP");
    }

    #[test]
    fn digit_only_prefixes_are_kept() {
        assert_eq!(derive_lp_token_symbol(Some(["12", "34"])), "12-34-LP");
        assert!(is_mintable_cw20_lp_symbol("12-34-LP"));
    }
}
