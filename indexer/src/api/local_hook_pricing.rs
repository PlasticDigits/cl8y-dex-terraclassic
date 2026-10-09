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
    inverse: std::collections::HashMap<String, HookInverse>,
}

/// Net(N) = quotient * retained_bps + net(remainder), with period 10,000.
/// Multiple independent floors need not be monotone, so inverse sizing uses
/// precomputed first-reaching residues rather than a binary search on net(N).
#[derive(Debug, Clone)]
struct HookInverse {
    retained_bps: u128,
    // A bounded immutable segment tree supports first-reaching residues after
    // any AMM-reachable original amount, including nonmonotone collisions.
    maxima: std::sync::Arc<[u16]>,
}
impl HookInverse {
    const WIDTH: usize = 16_384;
    fn build(fees: &[u16]) -> Option<Self> {
        let total: u32 = fees.iter().map(|f| u32::from(*f)).sum();
        let retained_bps = 10_000u32.checked_sub(total)? as u128;
        let mut maxima = vec![0; Self::WIDTH * 2];
        for r in 0..10_000u32 {
            maxima[Self::WIDTH + r as usize] =
                (r - fees.iter().map(|f| r * u32::from(*f) / 10_000).sum::<u32>()) as u16;
        }
        for node in (1..Self::WIDTH).rev() {
            maxima[node] = maxima[node * 2].max(maxima[node * 2 + 1]);
        }
        Some(Self {
            retained_bps,
            maxima: maxima.into(),
        })
    }
    fn first(
        &self,
        node: usize,
        start: usize,
        end: usize,
        lower: usize,
        target: u128,
    ) -> Option<u128> {
        if end <= lower || start >= 10_000 || u128::from(self.maxima[node]) < target {
            return None;
        }
        if end - start == 1 {
            return Some(start as u128);
        }
        let middle = start + (end - start) / 2;
        self.first(node * 2, start, middle, lower, target)
            .or_else(|| self.first(node * 2 + 1, middle, end, lower, target))
    }
    fn gross_from(&self, target: u128, lower: u128) -> Option<u128> {
        if target == 0 {
            return None;
        }
        let maximum = u128::from(self.maxima[1]);
        let first_quotient = if target <= maximum {
            0
        } else {
            if self.retained_bps == 0 {
                return None;
            }
            let difference = target - maximum;
            difference / self.retained_bps + u128::from(difference % self.retained_bps != 0)
        };
        let lower_quotient = lower / 10_000;
        let mut quotient = first_quotient.max(lower_quotient);
        let residue_start = if quotient == lower_quotient {
            (lower % 10_000) as usize
        } else {
            0
        };
        let threshold = target.saturating_sub(quotient.checked_mul(self.retained_bps)?);
        let residue = if let Some(residue) = self.first(1, 0, Self::WIDTH, residue_start, threshold)
        {
            residue
        } else {
            quotient = quotient.checked_add(1)?;
            let threshold = target.saturating_sub(quotient.checked_mul(self.retained_bps)?);
            self.first(1, 0, Self::WIDTH, 0, threshold)?
        };
        quotient.checked_mul(10_000)?.checked_add(residue)
    }
    fn gross(&self, target: u128) -> Option<u128> {
        self.gross_from(target, 0)
    }
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
        let tokens: HashSet<_> = deductions.iter().map(|d| d.token.clone()).collect();
        let inverse = tokens
            .into_iter()
            .filter_map(|token| {
                let fees = deductions
                    .iter()
                    .filter(|d| d.token == token)
                    .map(|d| d.bps)
                    .collect::<Vec<_>>();
                HookInverse::build(&fees).map(|prices| (token, prices))
            })
            .collect();
        Ok(Self {
            inverse,
            deductions,
            observation_identity: hooks.to_vec(),
        })
    }

    /// Minimal original post-commission output delivering the requested net.
    /// Tables are prepared during observation, never by a quote request.
    pub fn inverse_return(&self, ask: &str, target: u128) -> Result<u128, &'static str> {
        if !self.deductions.iter().any(|d| d.token == ask) {
            return (target > 0).then_some(target).ok_or("zero hook target");
        }
        self.inverse
            .get(ask)
            .and_then(|p| p.gross(target))
            .ok_or("hook inverse unavailable")
    }

    pub fn inverse_return_from(
        &self,
        ask: &str,
        target: u128,
        lower: u128,
    ) -> Result<u128, &'static str> {
        if !self.deductions.iter().any(|d| d.token == ask) {
            return (target > 0)
                .then_some(target.max(lower))
                .ok_or("zero hook target");
        }
        self.inverse
            .get(ask)
            .and_then(|p| p.gross_from(target, lower))
            .ok_or("hook inverse unavailable")
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
    #[test]
    fn inverse_is_exact_across_nonmonotone_individual_floor_collisions() {
        let prices = CachedHookPrices::from_observation(&[
            ("a".into(), tax("ask", 3333)),
            ("b".into(), tax("ask", 3333)),
            ("c".into(), tax("ask", 3333)),
        ])
        .unwrap();
        assert!(prices.net_return("ask", 4).unwrap() < prices.net_return("ask", 3).unwrap());
        let mut maximum = 0;
        for original in 1..100_000u128 {
            let net = prices.net_return("ask", original).unwrap();
            if net > maximum {
                for target in maximum + 1..=net {
                    assert_eq!(prices.inverse_return("ask", target).unwrap(), original);
                }
                maximum = net;
            }
        }
        let single = CachedHookPrices::from_observation(&[("a".into(), tax("ask", 333))]).unwrap();
        let gross = single.inverse_return("ask", 1_000_000).unwrap();
        assert!(single.net_return("ask", gross).unwrap() >= 1_000_000);
        assert!(single.net_return("ask", gross - 1).unwrap() < 1_000_000);
        assert!(single.inverse_return("ask", u128::MAX).is_err());
    }
    #[test]
    fn inverse_full_deduction_and_overdraw_fail_closed() {
        let full = CachedHookPrices::from_observation(&[("a".into(), tax("ask", 10_000))]).unwrap();
        assert!(full.inverse_return("ask", 1).is_err());
        let rounding = CachedHookPrices::from_observation(&[
            ("a".into(), tax("ask", 5000)),
            ("b".into(), tax("ask", 5000)),
        ])
        .unwrap();
        assert_eq!(rounding.inverse_return("ask", 1).unwrap(), 1);
        assert!(rounding.inverse_return("ask", 2).is_err());
        let overdraw = CachedHookPrices::from_observation(&[
            ("a".into(), tax("ask", 10_000)),
            ("b".into(), tax("ask", 1)),
        ])
        .unwrap();
        assert!(overdraw.inverse_return("ask", 1).is_err());
    }
    #[test]
    fn inverse_from_skips_unreachable_residues_without_assuming_monotone_net() {
        let prices = CachedHookPrices::from_observation(&[
            ("tax".into(), tax("ask", 2500)),
            (
                "burn".into(),
                json!({"burn_token":"ask","burn_percentage_bps":2500}),
            ),
        ])
        .unwrap();
        for lower in 0..500u128 {
            for target in 1..10 {
                let expected = (lower..lower + 100)
                    .find(|v| prices.net_return("ask", *v).unwrap() >= target)
                    .unwrap();
                assert_eq!(
                    prices.inverse_return_from("ask", target, lower).unwrap(),
                    expected
                );
            }
        }
        assert_eq!(prices.inverse_return_from("ask", 3, 4).unwrap(), 5);
    }
}
