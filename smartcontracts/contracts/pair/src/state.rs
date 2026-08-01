use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use dex_common::oracle::Observation;
use dex_common::pair::{LimitOrderSide, OrderStatusReason, OwnerOrderState};
use dex_common::types::{AssetInfo, FeeConfig};

/// Governance-configurable cap on batch/ladder placement size (GitLab #206).
#[cw_serde]
pub struct LimitOrderConfig {
    pub max_batch_rungs: u32,
}

pub const LIMIT_ORDER_CONFIG: Item<LimitOrderConfig> = Item::new("limit_order_cfg");

/// Governance dust thresholds for permissionless `CleanLimitBook` (GitLab #263).
#[cw_serde]
pub struct LimitCleanConfig {
    pub min_remaining_token0: Uint128,
    pub min_remaining_token1: Uint128,
}

pub const LIMIT_CLEAN_CONFIG: Item<LimitCleanConfig> = Item::new("limit_clean_cfg");

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

/// Cached CL8Y fee-discount lookup per `(trader, sender)` ([GitLab #251](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/251)).
#[cw_serde]
pub struct DiscountCacheEntry {
    pub effective_fee_bps: u16,
    pub discount: dex_common::fee_discount::DiscountResponse,
    pub cached_at: u64,
}

pub const DISCOUNT_CACHE: Map<(&Addr, &Addr), DiscountCacheEntry> = Map::new("disc_cache");

/// Next limit order id (monotonic).
pub const ORDER_NEXT_ID: Item<u64> = Item::new("order_next_id");
/// Doubly-linked limit orders (see [`crate::orderbook`]).
pub const ORDERS: Map<u64, LimitOrder> = Map::new("limit_orders");
pub const HEAD_BID: Item<Option<u64>> = Item::new("head_bid");
pub const HEAD_ASK: Item<Option<u64>> = Item::new("head_ask");
/// CW20 balances held for bids (token1) and asks (token0), excluded from reserves and sweep.
pub const PENDING_ESCROW_TOKEN0: Item<Uint128> = Item::new("escrow_t0");
pub const PENDING_ESCROW_TOKEN1: Item<Uint128> = Item::new("escrow_t1");

/// Refund owed when a limit order was removed from the book for expiry during a match walk.
/// `PENDING_ESCROW_TOKEN0` / `PENDING_ESCROW_TOKEN1` still include `remaining` until
/// `ClaimExpiredLimitOrder` transfers funds and subtracts pending.
#[cw_serde]
pub struct ExpiredLimitRefund {
    pub owner: Addr,
    pub side: LimitOrderSide,
    pub remaining: Uint128,
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub reason: Option<OrderStatusReason>,
}

pub const EXPIRED_LIMIT_CLAIMS: Map<u64, ExpiredLimitRefund> = Map::new("exp_limit_cl");

/// Live owner custody keyed by `(owner, order_id)`. The key remains stable when an order parks.
#[cw_serde]
pub struct OwnerOrderRecord {
    pub state: OwnerOrderState,
    pub side: LimitOrderSide,
    pub price: Option<Decimal>,
    pub remaining: Uint128,
    pub expires_at: Option<u64>,
    pub reason: Option<OrderStatusReason>,
}

pub const OWNER_ORDERS: Map<(&Addr, u64), OwnerOrderRecord> = Map::new("owner_orders_v1");
pub const OWNER_INDEX_READY: Item<bool> = Item::new("owner_idx_ready_v1");
pub const OWNER_INDEX_GENERATION: Item<u64> = Item::new("owner_idx_gen_v1");

#[cw_serde]
pub enum OwnerIndexBackfillPhase {
    Active,
    ParkedRefund,
}

#[cw_serde]
pub struct OwnerIndexBackfillCursor {
    pub phase: OwnerIndexBackfillPhase,
    /// Exclusive order-id cursor within the current phase.
    pub last_order_id: Option<u64>,
    /// Migration-time inclusive boundary. Legacy draft cursors capture it on first continuation.
    #[serde(default)]
    pub max_order_id: Option<u64>,
}

pub const OWNER_INDEX_BACKFILL_CURSOR: Item<OwnerIndexBackfillCursor> =
    Item::new("owner_idx_cursor_v1");

/// Prospective terminal lifecycle record. Pre-upgrade terminal orders intentionally have no row.
#[cw_serde]
pub struct OrderTombstone {
    pub owner: Addr,
    pub side: LimitOrderSide,
    pub price: Option<Decimal>,
    pub remaining: Uint128,
    pub expires_at: Option<u64>,
    pub reason: OrderStatusReason,
    pub terminal_height: u64,
    pub terminal_time: u64,
}

pub const ORDER_TOMBSTONES: Map<u64, OrderTombstone> = Map::new("order_tomb_v1");

/// Scope for per-transaction swap ordinals (GitLab #331).
#[cw_serde]
pub struct TxSwapScope {
    pub height: u64,
    pub tx_index: u32,
}

/// Resets when [`TxSwapScope`] changes; counts completed swaps in the current tx.
pub const TX_SWAP_SCOPE: Item<TxSwapScope> = Item::new("tx_swap_scope");
pub const TX_SWAP_COUNTER: Item<u32> = Item::new("tx_swap_counter");

#[cw_serde]
pub struct LimitOrder {
    pub owner: Addr,
    pub price: Decimal,
    pub remaining: Uint128,
    pub side: LimitOrderSide,
    /// `None` = never expires (also default for orders stored before this field existed).
    #[serde(default)]
    pub expires_at: Option<u64>,
    pub prev: Option<u64>,
    pub next: Option<u64>,
}

#[cfg(test)]
mod compatibility_tests {
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Storage;

    use super::*;

    #[test]
    fn expired_limit_refund_loads_pre_1_9_row_without_new_fields() {
        let mut deps = mock_dependencies();
        let order_id = 41u64;
        let legacy_json =
            br#"{"owner":"legacy-maker","side":"ask","remaining":"123456","expires_at":987654}"#;
        let key = EXPIRED_LIMIT_CLAIMS.key(order_id);
        deps.storage.set(&key, legacy_json);

        let row = EXPIRED_LIMIT_CLAIMS.load(&deps.storage, order_id).unwrap();
        assert_eq!(row.owner, Addr::unchecked("legacy-maker"));
        assert_eq!(row.side, LimitOrderSide::Ask);
        assert_eq!(row.remaining, Uint128::new(123_456));
        assert_eq!(row.expires_at, Some(987_654));
        assert_eq!(row.price, None);
        assert_eq!(row.reason, None);
    }
}
