//! Explicit recovery for legacy sisters managed by this token (#1429).
//! Settings dirty detection and permissionless skim never call this path.
use cosmwasm_std::{to_json_binary, Addr, DepsMut, Env, MessageInfo, Response, WasmMsg};
use dex_common::autolp::LpPolicy;
use dex_common::pair::{PoolResponse, QueryMsg as PairQuery};
use dex_common::types::{AssetInfo, PairInfo};

use crate::error::ContractError;
use crate::msg::Sku;
use crate::state::{CONFIG, FEATURES};

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum SisterQuery {
    GetConfig {},
    GetLpPolicy {},
}

#[derive(serde::Deserialize)]
struct SisterConfig {
    token: Addr,
    manager: Addr,
    factory: Addr,
    pair: Option<Addr>,
    quote_token: Option<Addr>,
    skimming: bool,
}

pub fn refresh(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    expected: LpPolicy,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.manager {
        return Err(ContractError::Unauthorized {});
    }
    if !FEATURES.load(deps.storage)?.auto_v2_lp {
        return Err(ContractError::SkuNotUnlocked {
            sku: Sku::AutoV2Lp.as_str().into(),
        });
    }
    // This operation neither invoices nor accepts deposits.
    if !info.funds.is_empty() {
        return Err(ContractError::InvalidAutolpPolicy {});
    }
    for address in [&expected.pair, &expected.quote, &expected.lp_token] {
        deps.api.addr_validate(address.as_str())?;
    }
    let sister = cfg.autolp.ok_or(ContractError::InvalidAutolpPolicy {})?;
    let current: SisterConfig = deps
        .querier
        .query_wasm_smart(&sister, &SisterQuery::GetConfig {})?;
    if current.token != env.contract.address
        || current.manager != env.contract.address
        || current.factory != cfg.factory
        || current.pair.as_ref() != Some(&expected.pair)
        || current.quote_token.as_ref() != Some(&expected.quote)
        || current.skimming
        || expected.tax_reserve.is_zero()
        || expected.quote_reserve.is_zero()
        || expected.total_share.is_zero()
    {
        return Err(ContractError::InvalidAutolpPolicy {});
    }
    // A pre-policy sister must be migrated first. Unsupported/malformed queries
    // fail; an explicitly missing legacy anchor (null) is recoverable.
    let _: Option<LpPolicy> = deps
        .querier
        .query_wasm_smart(&sister, &SisterQuery::GetLpPolicy {})?;
    let pair: PairInfo = deps
        .querier
        .query_wasm_smart(&expected.pair, &PairQuery::Pair {})?;
    let pool: PoolResponse = deps
        .querier
        .query_wasm_smart(&expected.pair, &PairQuery::Pool {})?;
    let tax = AssetInfo::Token {
        contract_addr: env.contract.address.to_string(),
    };
    let quote = AssetInfo::Token {
        contract_addr: expected.quote.to_string(),
    };
    let reserves = match (&pool.assets[0].info, &pool.assets[1].info) {
        (a, b) if a == &tax && b == &quote => (pool.assets[0].amount, pool.assets[1].amount),
        (a, b) if a == &quote && b == &tax => (pool.assets[1].amount, pool.assets[0].amount),
        _ => return Err(ContractError::InvalidAutolpPolicy {}),
    };
    if pair.contract_addr != expected.pair
        || pair.liquidity_token != expected.lp_token
        || reserves != (expected.tax_reserve, expected.quote_reserve)
        || pool.total_share != expected.total_share
    {
        return Err(ContractError::InvalidAutolpPolicy {});
    }

    // Only the already-bound pair is explicit. Sister authorization, immutable
    // factory validation, anchor capture, lock and registration all still run.
    // Normal message failure propagates and atomically reverts every callback.
    #[derive(serde::Serialize)]
    struct PairBind {
        pair: String,
    }
    #[derive(serde::Serialize)]
    struct SisterExecute {
        update_config: PairBind,
    }
    Ok(Response::new()
        .add_attribute("action", "refresh_autolp_policy")
        .add_attribute("autolp", sister.as_str())
        .add_attribute("pair", expected.pair.as_str())
        .add_message(WasmMsg::Execute {
            contract_addr: sister.into_string(),
            msg: to_json_binary(&SisterExecute {
                update_config: PairBind {
                    pair: expected.pair.into_string(),
                },
            })?,
            funds: vec![],
        }))
}
