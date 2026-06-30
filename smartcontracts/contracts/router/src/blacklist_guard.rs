use cosmwasm_std::{Addr, Deps};

use crate::error::ContractError;
use crate::msg::SwapOperation;
use dex_common::blacklist::BlacklistCheck;
use dex_common::factory::QueryMsg as FactoryQueryMsg;

pub fn assert_router_swap_not_blacklisted(
    deps: Deps,
    factory: &Addr,
    sender: &Addr,
    input_token: &Addr,
    operations: &[SwapOperation],
) -> Result<(), ContractError> {
    let mut tokens = vec![input_token.to_string()];
    let mut pairs = Vec::new();

    for op in operations {
        match op {
            SwapOperation::NativeSwap { .. } => {
                return Err(ContractError::NativeSwapNotSupported {});
            }
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
                ..
            } => {
                let offer = offer_asset_info
                    .assert_is_token()
                    .map_err(|_| ContractError::NativeTokenNotSupported {})?;
                let ask = ask_asset_info
                    .assert_is_token()
                    .map_err(|_| ContractError::NativeTokenNotSupported {})?;
                if !tokens.contains(&offer.to_string()) {
                    tokens.push(offer.to_string());
                }
                if !tokens.contains(&ask.to_string()) {
                    tokens.push(ask.to_string());
                }

                let pair_response: dex_common::factory::PairResponse = deps
                    .querier
                    .query_wasm_smart(
                        factory.to_string(),
                        &FactoryQueryMsg::Pair {
                            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
                        },
                    )
                    .map_err(|_| ContractError::PairNotFound {})?;
                let pair_addr = pair_response.pair.contract_addr.to_string();
                if !pairs.contains(&pair_addr) {
                    pairs.push(pair_addr);
                }
            }
        }
    }

    let resp: dex_common::blacklist::BlacklistCheckResponse = match deps.querier.query_wasm_smart(
        factory.to_string(),
        &FactoryQueryMsg::BlacklistCheck(BlacklistCheck {
            wallet: Some(sender.to_string()),
            tokens,
            pair: None,
            pairs,
        }),
    ) {
        Ok(r) => r,
        // GitLab #456 (SEC-I03 F02): only fail open for a genuinely pre-1.5.0 factory. A
        // reachable >=1.5.0 factory (or an unreadable/stale factory address) that errors here
        // is anomalous, so block rather than silently disabling blacklist enforcement.
        Err(_) => {
            if dex_common::blacklist::blacklist_query_error_blocks(&deps.querier, factory) {
                return Err(ContractError::BlacklistGuardUnavailable {});
            }
            return Ok(());
        }
    };

    if resp.blocked {
        return Err(ContractError::Blacklisted {});
    }

    Ok(())
}
