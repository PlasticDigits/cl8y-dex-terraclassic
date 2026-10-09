//! Immutable hook prices populated by scheduled ingestion, never request-time queries.
//! Mirrors dex-common::hook_settlement: each deduction floors against the original
//! post-commission ask output; supported LP hooks do not deduct that output.

use std::collections::HashSet;

use serde_json::Value;

const MAX_HOOKS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedHookDeduction {
    pub token: String,
    pub bps: u16,
}

#[derive(Debug, Clone, Default)]
pub struct CachedHookPrices {
    deductions: Vec<CachedHookDeduction>,
    // Preserve dispatch order and all config fields, including non-deducting LP
    // hooks. A governance/address/config change invalidates the quote generation.
    observation_identity: Vec<(String, Value)>,
}

fn address(value: &Value, field: &str) -> Result<String, &'static str> {
    value[field]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 128 && s.trim() == *s)
        .map(str::to_owned)
        .ok_or("unsupported hook pricing")
}

fn bps(value: &Value, field: &str) -> Result<u16, &'static str> {
    value[field]
        .as_u64()
        .filter(|n| *n <= 10_000)
        .map(|n| n as u16)
        .ok_or("unsupported hook pricing")
}

impl CachedHookPrices {
    pub fn observation_identity(&self) -> &[(String, Value)] {
        &self.observation_identity
    }

    pub fn has_deductions(&self, ask: &str) -> bool {
        self.deductions.iter().any(|d| d.token == ask && d.bps > 0)
    }
    /// Every configured hook must have a successful same-generation GetConfig.
    /// An absent response is an error, not an empty zero-fee policy.
    pub fn from_observation(hooks: &[(String, Value)]) -> Result<Self, &'static str> {
        if hooks.len() > MAX_HOOKS {
            return Err("invalid hook list");
        }
        let mut seen = HashSet::new();
        let mut deductions = Vec::new();
        for (hook, cfg) in hooks {
            if hook.is_empty() || hook.len() > 128 || !seen.insert(hook) {
                return Err("invalid hook list");
            }
            // cw_serde's HookConfigResponse rejects unknown fields and malformed
            // optional admin values even though admin is not part of fee math.
            let fields = cfg.as_object().ok_or("unsupported hook pricing")?;
            if fields.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "admin"
                        | "recipient"
                        | "tax_percentage_bps"
                        | "tax_token"
                        | "burn_token"
                        | "burn_percentage_bps"
                        | "target_pair"
                        | "lp_token"
                        | "percentage_bps"
                )
            }) || fields
                .get("admin")
                .is_some_and(|v| !v.is_null() && !v.is_string())
            {
                return Err("unsupported hook pricing");
            }
            let present = |keys: &[&str]| keys.iter().any(|key| !cfg[*key].is_null());
            let tax = present(&["recipient", "tax_percentage_bps", "tax_token"]);
            let burn = present(&["burn_token", "burn_percentage_bps"]);
            let lp = present(&["target_pair", "lp_token", "percentage_bps"]);
            let (token, fee) = match (tax, burn, lp) {
                (true, false, false) => {
                    address(cfg, "recipient")?;
                    (address(cfg, "tax_token")?, bps(cfg, "tax_percentage_bps")?)
                }
                (false, true, false) => (
                    address(cfg, "burn_token")?,
                    bps(cfg, "burn_percentage_bps")?,
                ),
                (false, false, true) => {
                    address(cfg, "target_pair")?;
                    address(cfg, "lp_token")?;
                    bps(cfg, "percentage_bps")?;
                    continue;
                }
                _ => return Err("unsupported hook pricing"),
            };
            deductions.push(CachedHookDeduction { token, bps: fee });
        }
        Ok(Self {
            deductions,
            observation_identity: hooks.to_vec(),
        })
    }

    pub fn net_return(&self, ask: &str, original: u128) -> Result<u128, &'static str> {
        let mut total_bps = 0u32;
        let mut total = 0u128;
        for fee in self.deductions.iter().filter(|fee| fee.token == ask) {
            total_bps += u32::from(fee.bps);
            if total_bps > 10_000 {
                return Err("hook deductions exceed output");
            }
            // Quotient/remainder decomposition is exact for every Uint128 input.
            let amount = original / 10_000 * u128::from(fee.bps)
                + original % 10_000 * u128::from(fee.bps) / 10_000;
            total = total
                .checked_add(amount)
                .ok_or("hook deductions exceed output")?;
        }
        original
            .checked_sub(total)
            .ok_or("hook deductions exceed output")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tax(token: &str, fee: u16) -> Value {
        json!({"recipient":"terra1recipient", "tax_token": token, "tax_percentage_bps":fee})
    }

    #[test]
    fn each_matching_hook_floors_against_original_and_other_tokens_do_not_deduct() {
        let prices = CachedHookPrices::from_observation(&[
            ("tax".into(), tax("ask", 3333)),
            (
                "burn".into(),
                json!({"burn_token":"ask","burn_percentage_bps":3333}),
            ),
            ("other".into(), tax("other-token", 10_000)),
            (
                "lp".into(),
                json!({"target_pair":"pair","lp_token":"lp","percentage_bps":100}),
            ),
        ])
        .unwrap();
        assert_eq!(prices.net_return("ask", 10).unwrap(), 4);
        assert_eq!(prices.net_return("unrelated", 10).unwrap(), 10);
        assert_eq!(prices.net_return("ask", 0).unwrap(), 0);
    }

    #[test]
    fn full_uint128_deduction_never_overflows_and_combined_bps_cannot_overdraw() {
        let prices =
            CachedHookPrices::from_observation(&[("tax".into(), tax("ask", 10_000))]).unwrap();
        assert_eq!(prices.net_return("ask", u128::MAX).unwrap(), 0);
        let prices = CachedHookPrices::from_observation(&[
            ("tax".into(), tax("ask", 10_000)),
            ("extra".into(), tax("ask", 1)),
        ])
        .unwrap();
        assert!(prices.net_return("ask", 1).is_err());
    }

    #[test]
    fn malformed_ambiguous_missing_and_out_of_range_config_fail_closed() {
        for cfg in [
            Value::Null,
            json!({}),
            json!({"tax_token":"ask"}),
            json!({"recipient":"r","tax_token":"ask","tax_percentage_bps":10_001}),
            json!({"recipient":"r","tax_token":"ask","tax_percentage_bps":"10"}),
            json!({"burn_token":"ask","burn_percentage_bps":1,"recipient":"r"}),
            json!({"target_pair":"p","lp_token":"lp"}),
            json!({"burn_token":"","burn_percentage_bps":1}),
            json!({"burn_token":"ask","burn_percentage_bps":1,"unknown":0}),
            json!({"burn_token":"ask","burn_percentage_bps":1,"admin":0}),
        ] {
            assert!(CachedHookPrices::from_observation(&[("hook".into(), cfg)]).is_err());
        }
    }

    #[test]
    fn eighth_hook_is_allowed_ninth_and_duplicate_fail() {
        let mut hooks: Vec<_> = (0..8)
            .map(|i| (format!("hook{i}"), tax("ask", 1)))
            .collect();
        assert!(CachedHookPrices::from_observation(&hooks).is_ok());
        hooks.push(("ninth".into(), tax("ask", 1)));
        assert!(CachedHookPrices::from_observation(&hooks).is_err());
        assert!(CachedHookPrices::from_observation(&[
            ("same".into(), tax("ask", 1)),
            ("same".into(), tax("ask", 1))
        ])
        .is_err());
    }

    #[test]
    fn non_deducting_hook_address_and_config_changes_have_distinct_identity() {
        let config = json!({"target_pair":"pair","lp_token":"lp","percentage_bps":100});
        let a = CachedHookPrices::from_observation(&[("hook-a".into(), config.clone())]).unwrap();
        let b = CachedHookPrices::from_observation(&[("hook-b".into(), config.clone())]).unwrap();
        let c = CachedHookPrices::from_observation(&[(
            "hook-a".into(),
            json!({"target_pair":"pair","lp_token":"lp","percentage_bps":200}),
        )])
        .unwrap();
        assert_eq!(
            a.net_return("ask", 100).unwrap(),
            b.net_return("ask", 100).unwrap()
        );
        assert_ne!(a.observation_identity(), b.observation_identity());
        assert_ne!(a.observation_identity(), c.observation_identity());
    }
}
