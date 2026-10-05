//! Governance trust lifecycle and bounded single-use route settlement (#1388).
use crate::{
    error::ContractError,
    state::{CONFIG, LISTED_PAIRS, PROTOCOL_EXEMPT},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, HexBinary, MessageInfo, Response, StdResult};
use cw_storage_plus::Item;
use dex_common::route_tax::{
    RouteTaxContextResponse, RouterConfigResponse, RouterQueryMsg, FUNDING_VERSION,
    MAX_SETTLEMENT_SLOTS, SETTLEMENT_VERSION,
};

/// Projection of full factory Config; unrelated config fields remain compatible.
#[derive(serde::Deserialize)]
struct FactoryGovernance {
    governance: Addr,
}

#[cw_serde]
pub struct SettlementConsumption {
    pub router: Addr,
    pub route_id: u64,
    pub slots: Vec<u32>,
}
/// One record per token, at most MAX_SETTLEMENT_SLOTS entries; never a growing map.
pub const SETTLEMENT_CONSUMPTION: Item<SettlementConsumption> =
    Item::new("route_settlement_consumption");

fn factory_governance(deps: Deps, factory: &Addr) -> StdResult<Addr> {
    let config: FactoryGovernance = deps
        .querier
        .query_wasm_smart(factory, &dex_common::factory::QueryMsg::Config {})?;
    Ok(config.governance)
}

pub fn rotate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    router: Option<String>,
    expected_code_id: Option<u64>,
    expected_checksum: Option<HexBinary>,
) -> Result<Response, ContractError> {
    let contract = deps
        .querier
        .query_wasm_contract_info(&env.contract.address)?;
    let mut config = CONFIG.load(deps.storage)?;
    // Factory governance can recover tokens owned by contract administrators that
    // expose migration only (such as CMM); no generic execute dispatcher is needed.
    let governance_authority = if contract.admin.as_deref() == Some(info.sender.as_str()) {
        None
    } else {
        let governance = factory_governance(deps.as_ref(), &config.factory)?;
        if info.sender != governance {
            return Err(ContractError::Unauthorized {});
        }
        Some(governance)
    };
    let candidate = router.map(|r| deps.api.addr_validate(&r)).transpose()?;
    let mut floor = 0;
    if let Some(candidate) = &candidate {
        let code_id = expected_code_id.ok_or(ContractError::RouteTaxContext {})?;
        let checksum = expected_checksum.ok_or(ContractError::RouteTaxContext {})?;
        let live = deps.querier.query_wasm_contract_info(candidate)?;
        let code = deps.querier.query_wasm_code_info(live.code_id)?;
        // Reviewed code alone does not establish custody: an attacker-owned copy
        // could migrate after selection. Trust immutable candidates or the actual
        // governance of the already configured factory, never token-manager custody.
        if let Some(admin) = &live.admin {
            let governance = match &governance_authority {
                Some(governance) => governance.clone(),
                None => factory_governance(deps.as_ref(), &config.factory)?,
            };
            if admin.as_str() != governance.as_str() {
                return Err(ContractError::RouteTaxContext {});
            }
        }
        let capability: RouterConfigResponse = deps
            .querier
            .query_wasm_smart(candidate, &RouterQueryMsg::Config {})?;
        // Expected artifact identity is selected by governance, never self-attested cw2.
        if checksum.len() != 32
            || live.code_id != code_id
            || code.checksum != checksum
            || capability.factory != config.factory
            || capability.funding_version != FUNDING_VERSION
            || capability.settlement_version != SETTLEMENT_VERSION
            || deps
                .querier
                .query_wasm_smart::<RouteTaxContextResponse>(
                    candidate,
                    &RouterQueryMsg::RouteTaxContext {},
                )
                .is_ok()
        {
            return Err(ContractError::RouteTaxContext {});
        }
        floor = capability.last_route_id;
        if let Some(consumed) = SETTLEMENT_CONSUMPTION.may_load(deps.storage)? {
            if consumed.router == *candidate {
                floor = floor.max(consumed.route_id);
            }
        }
    } else if expected_code_id.is_some() || expected_checksum.is_some() {
        return Err(ContractError::RouteTaxContext {});
    }
    // All validation precedes writes. Preserve the same address's replay floor.
    if let Some(previous) = &config.router {
        let overlaps = previous == env.contract.address
            || previous == config.factory
            || config.autolp.as_ref() == Some(previous)
            || LISTED_PAIRS
                .may_load(deps.storage, previous)?
                .unwrap_or(false);
        if !overlaps {
            PROTOCOL_EXEMPT.remove(deps.storage, previous);
        }
    }
    config.router = candidate.clone();
    if let Some(candidate) = &candidate {
        PROTOCOL_EXEMPT.save(deps.storage, candidate, &true)?;
        SETTLEMENT_CONSUMPTION.save(
            deps.storage,
            &SettlementConsumption {
                router: candidate.clone(),
                route_id: floor,
                slots: vec![],
            },
        )?;
    } else {
        SETTLEMENT_CONSUMPTION.remove(deps.storage);
    }
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "rotate_router")
        .add_attribute(
            "router",
            candidate
                .map(|a| a.to_string())
                .unwrap_or_else(|| "revoked".into()),
        ))
}

/// Consume a specific token-bound slot only after its authorized transfer passes checks.
pub fn consume(
    storage: &mut dyn cosmwasm_std::Storage,
    router: &Addr,
    ctx: &RouteTaxContextResponse,
    slot: u32,
) -> Result<(), ContractError> {
    if ctx.route_id == 0
        || ctx.transfers.len() > MAX_SETTLEMENT_SLOTS
        || slot as usize >= MAX_SETTLEMENT_SLOTS
    {
        return Err(ContractError::RouteTaxContext {});
    }
    let mut state = SETTLEMENT_CONSUMPTION
        .may_load(storage)?
        .unwrap_or(SettlementConsumption {
            router: router.clone(),
            route_id: 0,
            slots: vec![],
        });
    if state.router != *router
        || ctx.route_id < state.route_id
        || (ctx.route_id == state.route_id && state.slots.contains(&slot))
    {
        return Err(ContractError::RouteTaxContext {});
    }
    if ctx.route_id > state.route_id {
        state.route_id = ctx.route_id;
        state.slots.clear();
    } else if state.slots.is_empty() {
        // Empty state at the rotation baseline rejects every historical route.
        return Err(ContractError::RouteTaxContext {});
    }
    state.slots.push(slot);
    SETTLEMENT_CONSUMPTION.save(storage, &state)?;
    Ok(())
}
