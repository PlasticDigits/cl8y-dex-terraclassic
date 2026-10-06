//! Fee-hook settlement helpers shared by the pair contract and hook crates.
//!
//! Tax and burn hooks charge from the swap ask-token flow during pair
//! settlement (invariant **I-02**, GitLab #377). The pair queries each
//! registered hook's `GetConfig` and emits CW20 transfers before the net
//! return to the receiver.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Deps, StdError, StdResult, Uint128, Uint256, WasmMsg};
use cw20::Cw20ExecuteMsg;

/// Supported hook configuration shapes. Query once, and reject partial or ambiguous
/// responses rather than treating an unavailable fee as zero (#1398).
#[cw_serde]
#[derive(Default)]
struct HookConfigResponse {
    #[serde(default)]
    admin: Option<Addr>,
    #[serde(default)]
    recipient: Option<Addr>,
    #[serde(default)]
    tax_percentage_bps: Option<u16>,
    #[serde(default)]
    tax_token: Option<Addr>,
    #[serde(default)]
    burn_token: Option<Addr>,
    #[serde(default)]
    burn_percentage_bps: Option<u16>,
    #[serde(default)]
    target_pair: Option<Addr>,
    #[serde(default)]
    lp_token: Option<Addr>,
    #[serde(default)]
    percentage_bps: Option<u16>,
}

#[cw_serde]
enum HookQueryMsg {
    GetConfig {},
}

/// Fee slice computed from swap output and hook bps.
#[cw_serde]
pub struct HookFeeDeduction {
    pub hook: Addr,
    pub token: Addr,
    pub amount: Uint128,
    pub kind: HookFeeKind,
}

#[cw_serde]
pub enum HookFeeKind {
    Tax { recipient: Addr },
    Burn,
}

pub fn bps_of_amount(amount: Uint128, bps: u16) -> StdResult<Uint128> {
    if bps > 10_000 {
        return Err(StdError::generic_err(
            "unsupported hook pricing: bps exceeds 10000",
        ));
    }
    Ok(Uint128::try_from(
        Uint256::from(amount) * Uint256::from(bps as u128) / Uint256::from(10_000u128),
    )?)
}

/// Collect tax/burn deductions for registered hooks matching `ask_token`.
/// LP-burn hooks do not deduct the ask asset. Other hook pricing is unsupported;
/// governance must install an explicit supported pricing interface before use.
pub fn collect_fee_hook_deductions(
    deps: Deps,
    hooks: &[Addr],
    ask_token: &str,
    total_return: Uint128,
) -> StdResult<Vec<HookFeeDeduction>> {
    crate::hook::validate_stored_hooks(hooks)?;
    let ask = deps.api.addr_validate(ask_token)?;
    let mut out = Vec::new();
    let mut total_bps = 0u32;

    for hook in hooks {
        let cfg: HookConfigResponse = deps
            .querier
            .query_wasm_smart(hook, &HookQueryMsg::GetConfig {})
            .map_err(|_| {
                StdError::generic_err(format!(
                    "unsupported hook pricing: {hook} GetConfig unavailable"
                ))
            })?;
        let tax =
            cfg.recipient.is_some() || cfg.tax_percentage_bps.is_some() || cfg.tax_token.is_some();
        let burn = cfg.burn_token.is_some() || cfg.burn_percentage_bps.is_some();
        let lp =
            cfg.target_pair.is_some() || cfg.lp_token.is_some() || cfg.percentage_bps.is_some();
        let unsupported =
            || StdError::generic_err(format!("unsupported hook pricing: {hook} config"));
        let (token, bps, kind) = match (tax, burn, lp) {
            (true, false, false) => (
                deps.api
                    .addr_validate(cfg.tax_token.ok_or_else(unsupported)?.as_str())?,
                cfg.tax_percentage_bps.ok_or_else(unsupported)?,
                HookFeeKind::Tax {
                    recipient: deps
                        .api
                        .addr_validate(cfg.recipient.ok_or_else(unsupported)?.as_str())?,
                },
            ),
            (false, true, false) => (
                deps.api
                    .addr_validate(cfg.burn_token.ok_or_else(unsupported)?.as_str())?,
                cfg.burn_percentage_bps.ok_or_else(unsupported)?,
                HookFeeKind::Burn,
            ),
            (false, false, true) => {
                deps.api
                    .addr_validate(cfg.target_pair.ok_or_else(unsupported)?.as_str())?;
                deps.api
                    .addr_validate(cfg.lp_token.ok_or_else(unsupported)?.as_str())?;
                bps_of_amount(Uint128::zero(), cfg.percentage_bps.ok_or_else(unsupported)?)?;
                continue;
            }
            _ => return Err(unsupported()),
        };
        let amount = bps_of_amount(total_return, bps)?;
        if token == ask {
            total_bps += u32::from(bps);
            if total_bps > 10_000 {
                return Err(StdError::generic_err(
                    "hook deductions exceed output: total bps exceeds 10000",
                ));
            }
            if !amount.is_zero() {
                out.push(HookFeeDeduction {
                    hook: hook.clone(),
                    token,
                    amount,
                    kind,
                });
            }
        }
    }
    Ok(out)
}

/// Apply precisely the same integer fee deductions in quote and settlement.
pub fn net_return_after_hooks(
    total_return: Uint128,
    deductions: &[HookFeeDeduction],
) -> StdResult<Uint128> {
    let total = deductions
        .iter()
        .try_fold(Uint128::zero(), |acc, d| acc.checked_add(d.amount))?;
    Ok(total_return.checked_sub(total)?)
}

/// Build CW20 settlement transfers for fee hooks (tax → recipient, burn → hook).
pub fn fee_settlement_transfer_messages(deductions: &[HookFeeDeduction]) -> Vec<CosmosMsg> {
    let mut msgs = Vec::with_capacity(deductions.len());
    for d in deductions {
        let recipient = match &d.kind {
            HookFeeKind::Tax { recipient } => recipient.to_string(),
            HookFeeKind::Burn => d.hook.to_string(),
        };
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: d.token.to_string(),
            msg: cosmwasm_std::to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient,
                amount: d.amount,
            })
            .expect("cw20 transfer encoding"),
            funds: vec![],
        }));
    }
    msgs
}

#[cfg(test)]
mod tests {
    use super::bps_of_amount;
    use cosmwasm_std::Uint128;

    #[test]
    fn bps_math() {
        assert_eq!(
            bps_of_amount(Uint128::new(10_000), 500).unwrap(),
            Uint128::new(500)
        );
        assert_eq!(
            bps_of_amount(Uint128::new(1), 100).unwrap(),
            Uint128::zero()
        );
    }
}

#[cfg(test)]
#[path = "issue_1398_pricing_tests.rs"]
mod issue_1398_pricing_tests;
