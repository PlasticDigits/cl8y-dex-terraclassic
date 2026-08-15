//! LP CW20 ticker derivation for pair instantiate (GitLab #518).
//!
//! columbus-5 factory `lp_token_code_id` still uses classic Terraswap / cw20-base
//! validation: **`[a-zA-Z\-]{3,12}`** (no digits). Pair used to copy the first
//! four characters of each asset symbol into `{a}-{b}-LP`, so **UST1** and
//! **CL8Y** made `create_pair` revert at LP instantiate.
//!
//! PlasticDigits `cw20-mintable` (this workspace) already allows digits
//! (`[a-zA-Z0-9\-]{3,12}`). Sanitizing here keeps new pairs compatible with
//! **either** LP code — do not assume a factory `lp_token_code_id` upgrade.
//!
//! LP **name** and CosmWasm **label** keep the unsanitized symbols for display.

/// Classic Terraswap / cw20-base ticker charset used by columbus-5 LP code.
pub const CLASSIC_CW20_LP_SYMBOL_MIN_LEN: usize = 3;
pub const CLASSIC_CW20_LP_SYMBOL_MAX_LEN: usize = 12;

/// Letters taken from each asset symbol before joining `{a}-{b}-LP`.
pub const LP_SYMBOL_PREFIX_CHARS: usize = 4;

/// Digit-free fallback when prefixes collapse below the classic length floor.
/// Must itself satisfy [`is_classic_cw20_lp_symbol`] — do **not** use `CL8Y-LP`.
pub const FALLBACK_LP_TOKEN_SYMBOL: &str = "CLY-LP";

/// True when `symbol` matches classic `[a-zA-Z\-]{3,12}` (byte length).
pub fn is_classic_cw20_lp_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < CLASSIC_CW20_LP_SYMBOL_MIN_LEN || bytes.len() > CLASSIC_CW20_LP_SYMBOL_MAX_LEN
    {
        return false;
    }
    bytes.iter().all(|b| b.is_ascii_alphabetic() || *b == b'-')
}

/// ASCII letters only, up to `n` characters (digits and punctuation dropped).
pub fn alphabetic_prefix(symbol: &str, n: usize) -> String {
    symbol
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
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

/// LP ticker that always passes [`is_classic_cw20_lp_symbol`].
///
/// `None` (or prefixes that collapse to an invalid ticker) → [`FALLBACK_LP_TOKEN_SYMBOL`].
pub fn derive_lp_token_symbol(token_symbols: Option<[&str; 2]>) -> String {
    match token_symbols {
        Some([a, b]) => {
            let short_a = alphabetic_prefix(a, LP_SYMBOL_PREFIX_CHARS);
            let short_b = alphabetic_prefix(b, LP_SYMBOL_PREFIX_CHARS);
            let derived = format!("{}-{}-LP", short_a, short_b);
            let collapsed = collapse_hyphens(&derived);
            if is_classic_cw20_lp_symbol(&collapsed) {
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

/// Pre-#518 join (`take(4)` including digits). Used only in tests / docs.
pub fn legacy_unsanitized_lp_symbol(a: &str, b: &str) -> String {
    let short_a: String = a.chars().take(LP_SYMBOL_PREFIX_CHARS).collect();
    let short_b: String = b.chars().take(LP_SYMBOL_PREFIX_CHARS).collect();
    format!("{}-{}-LP", short_a, short_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_validator_rejects_digits_and_legacy_join() {
        assert!(!is_classic_cw20_lp_symbol("UST1-CUST-LP"));
        assert!(!is_classic_cw20_lp_symbol("UST1-cUST-LP"));
        assert!(!is_classic_cw20_lp_symbol("CL8Y-CLUN-LP"));
        assert!(!is_classic_cw20_lp_symbol("CL8Y-LP"));
        assert!(!is_classic_cw20_lp_symbol("AB"));
        assert!(!is_classic_cw20_lp_symbol("ABCDEFGHIJKLM"));
        assert!(!is_classic_cw20_lp_symbol("UST_CUST-LP"));
        assert_eq!(
            legacy_unsanitized_lp_symbol("UST1", "cUSTC"),
            "UST1-cUST-LP"
        );
        assert_eq!(
            legacy_unsanitized_lp_symbol("CL8Y", "cLUNC"),
            "CL8Y-cLUN-LP"
        );
        assert!(!is_classic_cw20_lp_symbol(&legacy_unsanitized_lp_symbol(
            "UST1", "cUSTC"
        )));
        assert!(!is_classic_cw20_lp_symbol(&legacy_unsanitized_lp_symbol(
            "CL8Y", "cLUNC"
        )));
    }

    #[test]
    fn classic_validator_accepts_letter_hyphen_tickers() {
        assert!(is_classic_cw20_lp_symbol("CLY-LP"));
        assert!(is_classic_cw20_lp_symbol("UST-CUST-LP"));
        assert!(is_classic_cw20_lp_symbol("CLY-CLUN-LP"));
        assert!(is_classic_cw20_lp_symbol("CLUN-CUST-LP"));
        assert!(is_classic_cw20_lp_symbol("AAA-BBB-LP"));
        assert!(is_classic_cw20_lp_symbol("AAAA-BBBB-LP"));
    }

    #[test]
    fn sanitizes_ust1_and_cl8y_launch_pairs() {
        assert_eq!(
            derive_lp_token_symbol(Some(["UST1", "cUSTC"])),
            "UST-cUST-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["UST1", "CUSTC"])),
            "UST-CUST-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["CL8Y", "cLUNC"])),
            "CLY-cLUN-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["CL8Y", "CLUNC"])),
            "CLY-CLUN-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["UST1", "USTR"])),
            "UST-USTR-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["cLUNC", "UST1"])),
            "cLUN-UST-LP"
        );
        assert_eq!(
            derive_lp_token_symbol(Some(["CLUNC", "UST1"])),
            "CLUN-UST-LP"
        );
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
        assert_eq!(
            derive_lp_token_symbol(Some(["cLUNC", "cUSTC"])),
            "cLUN-cUST-LP"
        );
    }

    #[test]
    fn fallback_when_missing_or_digit_only() {
        assert_eq!(derive_lp_token_symbol(None), FALLBACK_LP_TOKEN_SYMBOL);
        assert_eq!(
            derive_lp_token_symbol(Some(["12", "34"])),
            FALLBACK_LP_TOKEN_SYMBOL
        );
        assert!(is_classic_cw20_lp_symbol(FALLBACK_LP_TOKEN_SYMBOL));
        assert_ne!(FALLBACK_LP_TOKEN_SYMBOL, "CL8Y-LP");
    }

    #[test]
    fn instantiate_meta_keeps_full_symbols_on_name_and_label() {
        let (name, symbol, label) =
            lp_token_instantiate_meta(Some(&["UST1".into(), "CUSTC".into()]));
        assert_eq!(name, "UST1-CUSTC CL8YDEX LP");
        assert_eq!(symbol, "UST-CUST-LP");
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
}
