use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Item;

use crate::msg::SwapOperation;

pub const FACTORY: Item<Addr> = Item::new("factory");
pub const WRAP_MAPPER: Item<Addr> = Item::new("wrap_mapper");

#[cw_serde]
pub struct SwapState {
    pub sender: Addr,
    pub tax_context: dex_common::route_tax::RouteTaxContextResponse,
    pub refunds: Vec<dex_common::route_tax::RouteTransfer>,
    pub recipient: Addr,
    pub remaining_operations: Vec<SwapOperation>,
    pub minimum_receive: Option<Uint128>,
    pub output_token: Addr,
    /// Router CW20 balance of `output_token` immediately before the hop that will
    /// credit that token. Hop output is `balance_after - pre_hop_balance`.
    pub pre_hop_balance: Uint128,
    pub unwrap_output: bool,
    pub max_spread: Decimal,
}

pub const SWAP_STATE: Item<SwapState> = Item::new("swap_state");

/// Persists across in-place migrations; allocation and failure roll back together.
pub const LAST_ROUTE_ID: Item<u64> = Item::new("last_route_id");

pub fn allocate_route_id(storage: &mut dyn cosmwasm_std::Storage) -> cosmwasm_std::StdResult<u64> {
    let next = LAST_ROUTE_ID
        .may_load(storage)?
        .unwrap_or_default()
        .checked_add(1)
        .ok_or_else(|| cosmwasm_std::StdError::generic_err("route ID exhausted"))?;
    LAST_ROUTE_ID.save(storage, &next)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue1388_nonce_exhaustion_fails_without_mutating_persisted_counter() {
        let mut deps = cosmwasm_std::testing::mock_dependencies();
        assert_eq!(allocate_route_id(&mut deps.storage).unwrap(), 1);
        LAST_ROUTE_ID.save(&mut deps.storage, &u64::MAX).unwrap();
        assert!(allocate_route_id(&mut deps.storage)
            .unwrap_err()
            .to_string()
            .contains("route ID exhausted"));
        assert_eq!(LAST_ROUTE_ID.load(&deps.storage).unwrap(), u64::MAX);
    }
    #[test]
    fn issue1388_same_address_migration_preserves_counter_and_factory() {
        let mut deps = cosmwasm_std::testing::mock_dependencies();
        cw2::set_contract_version(&mut deps.storage, "cl8y-dex-router", "1.2.0").unwrap();
        LAST_ROUTE_ID.save(&mut deps.storage, &42).unwrap();
        FACTORY
            .save(&mut deps.storage, &Addr::unchecked("synthetic_factory"))
            .unwrap();
        crate::contract::migrate(
            deps.as_mut(),
            cosmwasm_std::testing::mock_env(),
            crate::msg::MigrateMsg {},
        )
        .unwrap();
        assert_eq!(LAST_ROUTE_ID.load(&deps.storage).unwrap(), 42);
        assert_eq!(
            FACTORY.load(&deps.storage).unwrap(),
            Addr::unchecked("synthetic_factory")
        );
        assert_eq!(allocate_route_id(&mut deps.storage).unwrap(), 43);
    }
}
