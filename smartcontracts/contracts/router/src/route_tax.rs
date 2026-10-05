//! Tax capability discovery and integer amount flow shared by execute and quote.
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Deps, StdError, StdResult, Uint128, WasmMsg};
use dex_common::{
    pair::HybridSwapParams,
    route_tax::{
        self, RouteTaxCapabilityResponse, RouteTaxKind, RouteTaxPreviewResponse, TokenQueryMsg,
    },
};

pub fn capability(deps: Deps, token: &Addr) -> StdResult<Option<RouteTaxCapabilityResponse>> {
    match deps.querier.query_wasm_smart::<RouteTaxCapabilityResponse>(
        token,
        &TokenQueryMsg::RouteTaxCapability {},
    ) {
        Ok(c) if c.funding_version == route_tax::FUNDING_VERSION => Ok(Some(c)),
        Ok(_) => Err(StdError::generic_err(
            "unsupported route-local tax funding version",
        )),
        Err(_) => {
            // A known legacy community/fee-on-transfer instance must never fall
            // through to a wallet-funded Send. Ordinary CW20s lack these APIs.
            for query in [
                br#"{"get_config":{}}"#.as_slice(),
                br#"{"tax_map":{}}"#.as_slice(),
                br#"{"tax_info":{}}"#.as_slice(),
            ] {
                let request = cosmwasm_std::QueryRequest::<cosmwasm_std::Empty>::Wasm(
                    cosmwasm_std::WasmQuery::Smart {
                        contract_addr: token.to_string(),
                        msg: query.into(),
                    },
                );
                if deps
                    .querier
                    .query::<serde::de::IgnoredAny>(&request)
                    .is_ok()
                {
                    return Err(StdError::generic_err(
                        "token route-local tax funding capability unavailable",
                    ));
                }
            }
            let raw = deps
                .querier
                .query_wasm_raw(token, b"contract_info")?
                .ok_or_else(|| StdError::generic_err("ordinary token identity unavailable"))?;
            let identity: cw2::ContractVersion = cosmwasm_std::from_json(raw)?;
            if !matches!(
                identity.contract.as_str(),
                "crates.io:cw20-base" | "crates.io:cw20-mintable" | "cw20-mintable"
            ) {
                return Err(StdError::generic_err(
                    "unsupported token trading capability",
                ));
            }
            Ok(None)
        }
    }
}

/// LG1390-1: capability, not current rates or guard settings, determines whether
/// a token may appear only once in [initial offer, each ask]. The adjacent buy
/// and sell of one intermediate is one logical occurrence, so its authenticated
/// cooldown continuation remains safe. Quotes and execution share this check.
pub fn validate_asset_path(
    deps: Deps,
    operations: &[crate::msg::SwapOperation],
    input_token: Option<&Addr>,
) -> StdResult<()> {
    let mut seen = Vec::<Addr>::new();
    let mut previous_ask: Option<Addr> = None;
    for op in operations {
        let crate::msg::SwapOperation::TerraSwap {
            offer_asset_info,
            ask_asset_info,
            ..
        } = op
        else {
            return Err(StdError::generic_err("Native swaps are not supported"));
        };
        let offer = deps
            .api
            .addr_validate(offer_asset_info.assert_is_token()?)?;
        let ask = deps.api.addr_validate(ask_asset_info.assert_is_token()?)?;
        if let Some(previous) = &previous_ask {
            if previous != offer {
                return Err(StdError::generic_err("disconnected swap asset path"));
            }
        } else {
            if input_token.is_some_and(|input| input != offer) {
                return Err(StdError::generic_err(
                    "swap input token does not match asset path",
                ));
            }
            seen.push(offer);
        }
        if seen.contains(&ask) && capability(deps, &ask)?.is_some() {
            return Err(StdError::generic_err(format!(
                "repeated route-tax-capable asset in swap path: {ask}"
            )));
        }
        seen.push(ask.clone());
        previous_ask = Some(ask);
    }
    Ok(())
}

pub fn preview(
    deps: Deps,
    token: &Addr,
    pair: &Addr,
    amount: Uint128,
    kind: RouteTaxKind,
    trader: Option<String>,
    router: Option<&Addr>,
) -> StdResult<RouteTaxPreviewResponse> {
    if let Some(c) = capability(deps, token)? {
        if router.is_some_and(|r| c.router.as_ref() != Some(r)) {
            return Err(StdError::generic_err(
                "token does not trust this route-local router",
            ));
        }
        let p: RouteTaxPreviewResponse = deps.querier.query_wasm_smart(
            token,
            &TokenQueryMsg::RouteTaxPreview {
                kind: kind.clone(),
                amount,
                trader,
                pair: pair.to_string(),
            },
        )?;
        if p.funding_version != route_tax::FUNDING_VERSION
            || p.gross != amount
            || p.tax > amount
            || p.net > amount
            || p.economic > amount
        {
            return Err(StdError::generic_err("invalid route-local tax preview"));
        }
        match kind {
            RouteTaxKind::Sell
                if p.economic.checked_add(p.tax)? <= amount && p.net == p.economic => {}
            RouteTaxKind::Buy if p.net.checked_add(p.tax)? == amount && p.economic == amount => {}
            _ => {
                return Err(StdError::generic_err(
                    "inconsistent route-local tax accounting",
                ))
            }
        }
        Ok(p)
    } else {
        Ok(RouteTaxPreviewResponse {
            funding_version: route_tax::FUNDING_VERSION,
            gross: amount,
            economic: amount,
            tax: Uint128::zero(),
            net: amount,
        })
    }
}

/// Recompute the book/pool allocation against the funded economic input. The
/// quoted gross ratio is retained; integer remainder belongs to the pool leg.
pub fn funded_hybrid(
    h: Option<HybridSwapParams>,
    gross: Uint128,
    economic: Uint128,
) -> StdResult<Option<HybridSwapParams>> {
    h.map(|mut h| {
        let total = h.pool_input.checked_add(h.book_input)?;
        if total == economic { return Ok(h); }
        if total != gross || gross.is_zero() { return Err(StdError::generic_err("hybrid pool_input + book_input must equal hop offer amount (gross or funded economic)")); }
        h.book_input = economic.multiply_ratio(h.book_input, gross);
        h.pool_input = economic.checked_sub(h.book_input)?;
        Ok(h)
    }).transpose()
}

pub fn transfer(
    deps: Deps,
    token: &Addr,
    recipient: &Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    let msg = if capability(deps, token)?.is_some() {
        to_json_binary(&route_tax::TokenExecuteMsg::RouteTransfer {
            recipient: recipient.to_string(),
            amount,
        })?
    } else {
        to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount,
        })?
    };
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token.to_string(),
        msg,
        funds: vec![],
    }))
}

pub fn reverse_buy(
    deps: Deps,
    token: &Addr,
    pair: &Addr,
    net: Uint128,
    trader: Option<String>,
    router: &Addr,
) -> StdResult<Uint128> {
    if net.is_zero() {
        return Ok(net);
    }
    let p = preview(
        deps,
        token,
        pair,
        Uint128::new(10_000),
        RouteTaxKind::Buy,
        trader,
        Some(router),
    )?;
    let denom = Uint128::new(10_000).checked_sub(p.tax)?;
    if denom.is_zero() {
        return Err(StdError::generic_err("buy tax leaves zero output"));
    }
    // Minimal G satisfying G-floor(G*bps/10000)>=net, exact at every floor boundary.
    net.checked_sub(Uint128::one())?
        .checked_multiply_ratio(10_000u128, denom.u128())
        .map_err(|e| StdError::generic_err(e.to_string()))?
        .checked_add(Uint128::one())
        .map_err(Into::into)
}
pub fn reverse_sell(
    deps: Deps,
    token: &Addr,
    pair: &Addr,
    economic: Uint128,
    trader: Option<String>,
    router: &Addr,
) -> StdResult<Uint128> {
    let Some(c) = capability(deps, token)? else {
        return Ok(economic);
    };
    let p = preview(
        deps,
        token,
        pair,
        Uint128::new(10_000 + u128::from(c.sell_bps)),
        RouteTaxKind::Sell,
        trader,
        Some(router),
    )?;
    economic
        .checked_add(economic.multiply_ratio(p.tax, Uint128::new(10_000)))
        .map_err(Into::into)
}

/// Pair reverse API has no greedy mode. Invert its actual forward greedy
/// engine instead of substituting pool-only economics. Both searches are
/// bounded by the 128-bit amount domain and preserve the maker-fill cap.
pub fn reverse_greedy_offer(
    deps: Deps,
    pair_addr: &Addr,
    offer_info: &dex_common::types::AssetInfo,
    target: Uint128,
    greedy: &dex_common::pair::GreedySwapParams,
    trader: Option<String>,
    sender: Option<String>,
) -> StdResult<Uint128> {
    if target.is_zero() {
        return Ok(Uint128::zero());
    }
    let output = |offer| -> StdResult<Uint128> {
        let sim: dex_common::pair::HybridSimulationResponse = deps.querier.query_wasm_smart(
            pair_addr,
            &dex_common::pair::QueryMsg::HybridSimulation {
                offer_asset: dex_common::types::Asset {
                    info: offer_info.clone(),
                    amount: offer,
                },
                hybrid: None,
                greedy: Some(greedy.clone()),
                trader: trader.clone(),
                sender: sender.clone(),
                belief_price: None,
            },
        )?;
        Ok(sim.return_amount)
    };
    let mut hi = target;
    let mut lo = Uint128::zero();
    for _ in 0..128 {
        if output(hi)? >= target {
            break;
        }
        lo = hi;
        hi = hi.checked_mul(Uint128::new(2)).unwrap_or(Uint128::MAX);
        if hi == lo {
            return Err(StdError::generic_err(
                "greedy reverse target cannot be funded",
            ));
        }
    }
    if output(hi)? < target {
        return Err(StdError::generic_err(
            "greedy reverse target cannot be funded",
        ));
    }
    while hi.u128() - lo.u128() > 1 {
        let mid = Uint128::new(lo.u128() + (hi.u128() - lo.u128()) / 2);
        if output(mid)? >= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Ok(hi)
}
