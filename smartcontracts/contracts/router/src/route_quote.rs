//! Immutable discovery shared only within one forward/reverse query (#1422).
//!
//! Reverse sizing and its mandatory fresh forward validation see the same chain
//! snapshot. Reuse capability and factory discovery, never amount-dependent tax
//! previews or pair simulations. Nothing is persisted or reused by execution.
use cosmwasm_std::{Addr, Deps, StdResult, Uint128};
use dex_common::{
    factory::{PairResponse, QueryMsg},
    route_tax::{RouteTaxCapabilityResponse, RouteTaxKind, RouteTaxPreviewResponse},
    types::AssetInfo,
};

pub(crate) struct QuoteContext<'a> {
    deps: Deps<'a>,
    factory: Addr,
    router: &'a Addr,
    capabilities: Vec<(Addr, Option<RouteTaxCapabilityResponse>)>,
    pairs: Vec<([AssetInfo; 2], PairResponse)>,
}

impl<'a> QuoteContext<'a> {
    pub fn new(deps: Deps<'a>, factory: Addr, router: &'a Addr) -> Self {
        Self {
            deps,
            factory,
            router,
            capabilities: vec![],
            pairs: vec![],
        }
    }

    fn capability(&mut self, token: &Addr) -> StdResult<Option<RouteTaxCapabilityResponse>> {
        if let Some((_, capability)) = self.capabilities.iter().find(|(asset, _)| asset == token) {
            return Ok(capability.clone());
        }
        // Cache ordinary CW20 absence only after all fail-closed identity checks
        // pass. A discovery error propagates; it never becomes an exemption.
        let capability = crate::route_tax::capability(self.deps, token)?;
        self.capabilities.push((token.clone(), capability.clone()));
        Ok(capability)
    }

    pub fn pair(&mut self, offer: &AssetInfo, ask: &AssetInfo) -> StdResult<PairResponse> {
        let assets = [offer.clone(), ask.clone()];
        if let Some((_, pair)) = self.pairs.iter().find(|(key, _)| key == &assets) {
            return Ok(pair.clone());
        }
        let pair: PairResponse = self.deps.querier.query_wasm_smart(
            &self.factory,
            &QueryMsg::Pair {
                asset_infos: assets.clone(),
            },
        )?;
        self.pairs.push((assets, pair.clone()));
        Ok(pair)
    }

    pub fn preview(
        &mut self,
        token: &Addr,
        pair: &Addr,
        amount: Uint128,
        kind: RouteTaxKind,
        trader: Option<String>,
    ) -> StdResult<RouteTaxPreviewResponse> {
        let capability = self.capability(token)?;
        crate::route_tax::preview_with_capability(
            self.deps,
            token,
            pair,
            amount,
            kind,
            trader,
            Some(self.router),
            capability.as_ref(),
        )
    }

    pub fn reverse_buy(
        &mut self,
        token: &Addr,
        pair: &Addr,
        net: Uint128,
        trader: Option<String>,
    ) -> StdResult<Uint128> {
        if net.is_zero() {
            return Ok(net);
        }
        let preview = self.preview(token, pair, Uint128::new(10_000), RouteTaxKind::Buy, trader)?;
        crate::route_tax::reverse_buy_amount(net, &preview)
    }

    pub fn reverse_sell(
        &mut self,
        token: &Addr,
        pair: &Addr,
        economic: Uint128,
        trader: Option<String>,
    ) -> StdResult<Uint128> {
        let Some(capability) = self.capability(token)? else {
            return Ok(economic);
        };
        let preview = crate::route_tax::preview_with_capability(
            self.deps,
            token,
            pair,
            Uint128::new(10_000 + u128::from(capability.sell_bps)),
            RouteTaxKind::Sell,
            trader,
            Some(self.router),
            Some(&capability),
        )?;
        crate::route_tax::reverse_sell_amount(economic, &preview)
    }
}
