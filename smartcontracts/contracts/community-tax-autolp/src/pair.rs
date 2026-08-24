//! Factory-listed pair with the tax token as one side (GitLab #610 / audit M-3).
//!
//! Same lookup as token `RegisterListedPair` (**T592-9**): query the candidate
//! for `Pair {}`, require this tax token in `asset_infos`, then confirm
//! `factory.Pair { asset_infos }` returns the same `contract_addr`.
//! Factory is the immutable launcher pin — do not trust a pair-reported factory.

use cosmwasm_std::{to_json_binary, Addr, Deps, QueryRequest, WasmQuery};
use dex_common::factory::{PairResponse, QueryMsg as FactoryQuery};
use dex_common::pair::QueryMsg as PairQuery;
use dex_common::types::{AssetInfo, PairInfo};

use crate::error::ContractError;

/// Validate `pair_raw` against `factory` and `tax_token`.
///
/// Returns `(factory-listed pair, quote CW20 if the other side is a token)`.
/// Native quote → `quote = None` (provide path already skips without a CW20 quote).
pub fn require_factory_listed_tax_pair(
    deps: Deps,
    factory: &Addr,
    tax_token: &Addr,
    pair_raw: &str,
) -> Result<(Addr, Option<Addr>), ContractError> {
    let pair = deps.api.addr_validate(pair_raw)?;
    let pair_info: PairInfo = deps
        .querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: pair.to_string(),
            msg: to_json_binary(&PairQuery::Pair {})?,
        }))
        .map_err(|_| ContractError::PairNotListed {})?;

    if pair_info.contract_addr != pair {
        return Err(ContractError::PairNotListed {});
    }

    let quote = quote_if_holds_tax(&pair_info.asset_infos, tax_token)?;

    let factory_pair: PairResponse = deps
        .querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: factory.to_string(),
            msg: to_json_binary(&FactoryQuery::Pair {
                asset_infos: pair_info.asset_infos.clone(),
            })?,
        }))
        .map_err(|_| ContractError::PairNotListed {})?;
    if factory_pair.pair.contract_addr != pair {
        return Err(ContractError::PairNotListed {});
    }

    Ok((factory_pair.pair.contract_addr, quote))
}

fn quote_if_holds_tax(
    assets: &[AssetInfo; 2],
    tax_token: &Addr,
) -> Result<Option<Addr>, ContractError> {
    let mut tax_idx: Option<usize> = None;
    for (i, a) in assets.iter().enumerate() {
        if let AssetInfo::Token { contract_addr } = a {
            if contract_addr == tax_token.as_str() {
                tax_idx = Some(i);
                break;
            }
        }
    }
    let Some(i) = tax_idx else {
        return Err(ContractError::PairNotListed {});
    };
    let other = &assets[1 - i];
    match other {
        AssetInfo::Token { contract_addr } => Ok(Some(Addr::unchecked(contract_addr.as_str()))),
        AssetInfo::NativeToken { .. } => Ok(None),
    }
}
