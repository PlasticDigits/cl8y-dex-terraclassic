//! Instantiate identity rules for community tax tokens (GitLab #604).
//!
//! Name/symbol are ASCII alphanumeric only. Decimals are **6–18**. Existing
//! columbus-5 **11611** instances keep their metadata; these checks apply to
//! **new** instantiates from the rotated `token_code_id` only.

use crate::error::ContractError;
use crate::msg::{
    MAX_DECIMALS, MAX_NAME_LEN, MAX_SYMBOL_LEN, MIN_DECIMALS, MIN_NAME_LEN, MIN_SYMBOL_LEN,
};

fn is_ascii_alnum(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Reject before `cw20_base::instantiate` so CLI/LCD cannot bypass the dApp.
pub fn validate_identity(name: &str, symbol: &str, decimals: u8) -> Result<(), ContractError> {
    if !(MIN_DECIMALS..=MAX_DECIMALS).contains(&decimals) {
        return Err(ContractError::DecimalsRange {
            min: MIN_DECIMALS,
            max: MAX_DECIMALS,
            got: decimals,
        });
    }
    if name.len() < MIN_NAME_LEN || name.len() > MAX_NAME_LEN || !is_ascii_alnum(name) {
        return Err(ContractError::InvalidName {});
    }
    if symbol.len() < MIN_SYMBOL_LEN || symbol.len() > MAX_SYMBOL_LEN || !is_ascii_alnum(symbol) {
        return Err(ContractError::InvalidSymbol {});
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bounds() {
        validate_identity("Abc", "ABC", 6).unwrap();
        validate_identity("DemoToken", "DEMO", 18).unwrap();
        validate_identity(&"A".repeat(50), &"B".repeat(12), 9).unwrap();
    }

    #[test]
    fn rejects_decimals() {
        assert!(matches!(
            validate_identity("Demo", "DEMO", 5),
            Err(ContractError::DecimalsRange {
                min: 6,
                max: 18,
                got: 5
            })
        ));
        assert!(matches!(
            validate_identity("Demo", "DEMO", 19),
            Err(ContractError::DecimalsRange {
                min: 6,
                max: 18,
                got: 19
            })
        ));
        assert!(matches!(
            validate_identity("Demo", "DEMO", 0),
            Err(ContractError::DecimalsRange { .. })
        ));
        assert!(matches!(
            validate_identity("Demo", "DEMO", 255),
            Err(ContractError::DecimalsRange { .. })
        ));
    }

    #[test]
    fn rejects_name_charset_and_len() {
        assert!(validate_identity("My Token", "DEMO", 6).is_err());
        assert!(validate_identity("Demo!", "DEMO", 6).is_err());
        assert!(validate_identity("🚀", "DEMO", 6).is_err());
        assert!(validate_identity("ab", "DEMO", 6).is_err());
        assert!(validate_identity("", "DEMO", 6).is_err());
        assert!(validate_identity("Demo-V2", "DEMO", 6).is_err());
        assert!(validate_identity(&"A".repeat(51), "DEMO", 6).is_err());
    }

    #[test]
    fn rejects_symbol_charset_and_len() {
        assert!(validate_identity("Demo", "DE-MO", 6).is_err());
        assert!(validate_identity("Demo", "demo_1", 6).is_err());
        assert!(validate_identity("Demo", "D", 6).is_err());
        assert!(validate_identity("Demo", "TOOLONGSYMBOLX", 6).is_err());
        assert!(validate_identity("Demo", "AB", 6).is_err());
    }

    #[test]
    fn preserves_name_case_check() {
        validate_identity("Demo", "demo", 6).unwrap();
        validate_identity("DeMo", "DeMo", 6).unwrap();
    }
}
