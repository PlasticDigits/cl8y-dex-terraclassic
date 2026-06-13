//! Fee-hook settlement helpers shared by the pair contract and hook crates.
//!
//! Tax and burn hooks charge from the swap ask-token flow during pair
//! settlement (invariant **I-02**, GitLab #377). The pair queries each
//! registered hook's `GetConfig` and emits CW20 transfers before the net
//! return to the receiver.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Deps, StdResult, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;

/// Tax-hook `GetConfig` response shape (subset used for settlement).
#[cw_serde]
pub struct TaxHookConfigView {
    pub recipient: Addr,
    pub tax_percentage_bps: u16,
    pub tax_token: Addr,
}

/// Burn-hook `GetConfig` response shape (subset used for settlement).
#[cw_serde]
pub struct BurnHookConfigView {
    pub burn_token: Addr,
    pub burn_percentage_bps: u16,
}

#[cw_serde]
enum TaxHookQueryMsg {
    #[serde(rename = "get_config")]
    GetConfig {},
}

#[cw_serde]
enum BurnHookQueryMsg {
    #[serde(rename = "get_config")]
    GetConfig {},
}

#[cw_serde]
struct TaxHookConfigResponse {
    recipient: Addr,
    tax_percentage_bps: u16,
    tax_token: Addr,
    admin: Addr,
}

#[cw_serde]
struct BurnHookConfigResponse {
    burn_token: Addr,
    burn_percentage_bps: u16,
    admin: Addr,
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
    Ok(amount
        .checked_mul(Uint128::from(bps as u128))?
        .checked_div(Uint128::new(10_000))?)
}

pub fn try_tax_hook_config(deps: Deps, hook: &Addr) -> StdResult<Option<TaxHookConfigView>> {
    let resp: TaxHookConfigResponse = match deps
        .querier
        .query_wasm_smart(hook, &TaxHookQueryMsg::GetConfig {})
    {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    Ok(Some(TaxHookConfigView {
        recipient: resp.recipient,
        tax_percentage_bps: resp.tax_percentage_bps,
        tax_token: resp.tax_token,
    }))
}

pub fn try_burn_hook_config(deps: Deps, hook: &Addr) -> StdResult<Option<BurnHookConfigView>> {
    let resp: BurnHookConfigResponse = match deps
        .querier
        .query_wasm_smart(hook, &BurnHookQueryMsg::GetConfig {})
    {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    Ok(Some(BurnHookConfigView {
        burn_token: resp.burn_token,
        burn_percentage_bps: resp.burn_percentage_bps,
    }))
}

/// Collect tax/burn deductions for registered hooks matching `ask_token`.
pub fn collect_fee_hook_deductions(
    deps: Deps,
    hooks: &[Addr],
    ask_token: &str,
    total_return: Uint128,
) -> StdResult<Vec<HookFeeDeduction>> {
    let ask = deps.api.addr_validate(ask_token)?;
    let mut out = Vec::new();

    for hook in hooks {
        if let Some(cfg) = try_tax_hook_config(deps, hook)? {
            if cfg.tax_token == ask {
                let amount = bps_of_amount(total_return, cfg.tax_percentage_bps)?;
                if !amount.is_zero() {
                    out.push(HookFeeDeduction {
                        hook: hook.clone(),
                        token: cfg.tax_token,
                        amount,
                        kind: HookFeeKind::Tax {
                            recipient: cfg.recipient,
                        },
                    });
                }
            }
        }
        if let Some(cfg) = try_burn_hook_config(deps, hook)? {
            if cfg.burn_token == ask {
                let amount = bps_of_amount(total_return, cfg.burn_percentage_bps)?;
                if !amount.is_zero() {
                    out.push(HookFeeDeduction {
                        hook: hook.clone(),
                        token: cfg.burn_token,
                        amount,
                        kind: HookFeeKind::Burn,
                    });
                }
            }
        }
    }

    Ok(out)
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
