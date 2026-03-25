use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use dex_common::oracle::Observation;
use dex_common::pair::LimitOrderSide;
use dex_common::types::{AssetInfo, FeeConfig};

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct PairInfoState {
    pub asset_infos: [AssetInfo; 2],
    pub lp_token: Addr,
    pub factory: Addr,
}

/// TWAP oracle state stored per pair.
#[cw_serde]
pub struct OracleState {
    /// Number of observation slots allocated.
    pub cardinality: u16,
    /// Index of the most recently written observation (wraps around).
    pub index: u16,
    /// How many slots have been written at least once (≤ cardinality).
    pub cardinality_initialized: u16,
}

pub const PAIR_INFO: Item<PairInfoState> = Item::new("pair_info");
pub const RESERVES: Item<(Uint128, Uint128)> = Item::new("reserves");
pub const FEE_CONFIG: Item<FeeConfig> = Item::new("fee_config");
pub const HOOKS: Item<Vec<Addr>> = Item::new("hooks");
pub const TOTAL_LP_SUPPLY: Item<Uint128> = Item::new("total_lp_supply");

pub const ORACLE_STATE: Item<OracleState> = Item::new("oracle_state");
/// Ring buffer of observations, keyed by slot index (0 .. cardinality-1).
pub const OBSERVATIONS: Map<u16, Observation> = Map::new("observations");

pub const PAUSED: Item<bool> = Item::new("paused");
pub const DISCOUNT_REGISTRY: Item<Option<Addr>> = Item::new("discount_registry");

/// Next limit order id (monotonic).
pub const ORDER_NEXT_ID: Item<u64> = Item::new("order_next_id");
/// Doubly-linked limit orders (see [`crate::orderbook`]).
pub const ORDERS: Map<u64, LimitOrder> = Map::new("limit_orders");
pub const HEAD_BID: Item<Option<u64>> = Item::new("head_bid");
pub const HEAD_ASK: Item<Option<u64>> = Item::new("head_ask");
/// CW20 balances held for bids (token1) and asks (token0), excluded from reserves and sweep.
pub const PENDING_ESCROW_TOKEN0: Item<Uint128> = Item::new("escrow_t0");
pub const PENDING_ESCROW_TOKEN1: Item<Uint128> = Item::new("escrow_t1");

#[cw_serde]
pub struct LimitOrder {
    pub owner: Addr,
    pub price: Decimal,
    pub remaining: Uint128,
    pub side: LimitOrderSide,
    pub prev: Option<u64>,
    pub next: Option<u64>,
}
