use cosmwasm_std::{Addr, Deps, Order, Response, StdError, StdResult, Storage};
use cw_storage_plus::Bound;
use dex_common::pair::{
    OrderStatus, OrderStatusReason, OrderStatusResponseV1, OwnerInventoryResponse,
    OwnerInventoryRow, OwnerInventorySnapshot, OwnerOrderState, PairApiFeature,
    PairProtocolResponse, DEFAULT_OWNER_INVENTORY_PAGE_SIZE, MAX_OWNER_INDEX_BACKFILL_LIMIT,
    MAX_OWNER_INVENTORY_PAGE_SIZE, ORDER_API_SCHEMA_VERSION,
};

use crate::error::ContractError;
use crate::state::{
    ExpiredLimitRefund, LimitOrder, OrderTombstone, OwnerIndexBackfillPhase, OwnerOrderRecord,
    EXPIRED_LIMIT_CLAIMS, ORDERS, ORDER_NEXT_ID, ORDER_TOMBSTONES, OWNER_INDEX_BACKFILL_CURSOR,
    OWNER_INDEX_GENERATION, OWNER_INDEX_READY, OWNER_ORDERS,
};

pub fn save_active(storage: &mut dyn Storage, order_id: u64, order: &LimitOrder) -> StdResult<()> {
    OWNER_ORDERS.save(
        storage,
        (&order.owner, order_id),
        &OwnerOrderRecord {
            state: OwnerOrderState::Active,
            side: order.side.clone(),
            price: Some(order.price),
            remaining: order.remaining,
            expires_at: order.expires_at,
            reason: None,
        },
    )
}

pub fn save_parked(
    storage: &mut dyn Storage,
    order_id: u64,
    row: &ExpiredLimitRefund,
) -> StdResult<()> {
    OWNER_ORDERS.save(
        storage,
        (&row.owner, order_id),
        &OwnerOrderRecord {
            state: OwnerOrderState::ParkedRefund,
            side: row.side.clone(),
            price: row.price,
            remaining: row.remaining,
            expires_at: row.expires_at,
            reason: row.reason.clone(),
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub fn save_terminal(
    storage: &mut dyn Storage,
    order_id: u64,
    owner: &Addr,
    side: dex_common::pair::LimitOrderSide,
    price: Option<cosmwasm_std::Decimal>,
    remaining: cosmwasm_std::Uint128,
    expires_at: Option<u64>,
    reason: OrderStatusReason,
    terminal_height: u64,
    terminal_time: u64,
) -> StdResult<()> {
    OWNER_ORDERS.remove(storage, (owner, order_id));
    ORDER_TOMBSTONES.save(
        storage,
        order_id,
        &OrderTombstone {
            owner: owner.clone(),
            side,
            price,
            remaining,
            expires_at,
            reason,
            terminal_height,
            terminal_time,
        },
    )
}

pub fn query_status(deps: Deps, order_id: u64) -> StdResult<OrderStatusResponseV1> {
    if order_id == 0 {
        return Err(StdError::generic_err("order_id must be positive"));
    }
    if let Some(order) = ORDERS.may_load(deps.storage, order_id)? {
        return Ok(OrderStatusResponseV1 {
            schema_version: ORDER_API_SCHEMA_VERSION,
            order_id,
            status: OrderStatus::Active,
            owner: Some(order.owner),
            side: Some(order.side),
            price: Some(order.price),
            remaining: Some(order.remaining),
            expires_at: order.expires_at,
            reason: None,
            terminal_height: None,
            terminal_time: None,
        });
    }
    if let Some(row) = EXPIRED_LIMIT_CLAIMS.may_load(deps.storage, order_id)? {
        return Ok(OrderStatusResponseV1 {
            schema_version: ORDER_API_SCHEMA_VERSION,
            order_id,
            status: OrderStatus::ParkedRefund,
            owner: Some(row.owner),
            side: Some(row.side),
            price: row.price,
            remaining: Some(row.remaining),
            expires_at: row.expires_at,
            reason: row.reason,
            terminal_height: None,
            terminal_time: None,
        });
    }
    if let Some(row) = ORDER_TOMBSTONES.may_load(deps.storage, order_id)? {
        let status = if row.reason == OrderStatusReason::FullyExecuted {
            OrderStatus::FullyExecuted
        } else {
            OrderStatus::Cancelled
        };
        return Ok(OrderStatusResponseV1 {
            schema_version: ORDER_API_SCHEMA_VERSION,
            order_id,
            status,
            owner: Some(row.owner),
            side: Some(row.side),
            price: row.price,
            remaining: Some(row.remaining),
            expires_at: row.expires_at,
            reason: Some(row.reason),
            terminal_height: Some(row.terminal_height),
            terminal_time: Some(row.terminal_time),
        });
    }
    Ok(OrderStatusResponseV1 {
        schema_version: ORDER_API_SCHEMA_VERSION,
        order_id,
        status: OrderStatus::NotFound,
        owner: None,
        side: None,
        price: None,
        remaining: None,
        expires_at: None,
        reason: None,
        terminal_height: None,
        terminal_time: None,
    })
}

pub fn query_owner_inventory(
    deps: Deps,
    owner: String,
    snapshot: Option<OwnerInventorySnapshot>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<OwnerInventoryResponse> {
    if !OWNER_INDEX_READY.load(deps.storage)? {
        return Err(StdError::generic_err(
            "owner inventory backfill is not ready",
        ));
    }
    let generation = OWNER_INDEX_GENERATION.load(deps.storage)?;
    let current_max_order_id = ORDER_NEXT_ID
        .load(deps.storage)?
        .checked_sub(1)
        .ok_or_else(|| StdError::generic_err("invalid zero next order id"))?;
    let snapshot = match snapshot {
        Some(snapshot) => {
            if snapshot.generation != generation {
                return Err(StdError::generic_err(
                    "owner inventory snapshot generation mismatch",
                ));
            }
            if snapshot.max_order_id > current_max_order_id {
                return Err(StdError::generic_err(
                    "owner inventory snapshot exceeds current order high-water",
                ));
            }
            snapshot
        }
        None => OwnerInventorySnapshot {
            generation,
            max_order_id: current_max_order_id,
        },
    };
    if start_after.is_some_and(|cursor| cursor > snapshot.max_order_id) {
        return Err(StdError::generic_err(
            "owner inventory cursor exceeds snapshot",
        ));
    }
    let limit = limit.unwrap_or(DEFAULT_OWNER_INVENTORY_PAGE_SIZE);
    if limit == 0 || limit > MAX_OWNER_INVENTORY_PAGE_SIZE {
        return Err(StdError::generic_err(format!(
            "owner inventory limit must be between 1 and {MAX_OWNER_INVENTORY_PAGE_SIZE}"
        )));
    }
    let owner = deps.api.addr_validate(&owner)?;
    let start = start_after.map(Bound::exclusive);
    let end = Some(Bound::inclusive(snapshot.max_order_id));
    let mut rows: Vec<OwnerInventoryRow> = OWNER_ORDERS
        .prefix(&owner)
        .range(deps.storage, start, end, Order::Ascending)
        .take(limit as usize + 1)
        .map(|item| {
            let (order_id, row) = item?;
            Ok(OwnerInventoryRow {
                order_id,
                owner: owner.clone(),
                state: row.state,
                side: row.side,
                price: row.price,
                remaining: row.remaining,
                expires_at: row.expires_at,
                reason: row.reason,
            })
        })
        .collect::<StdResult<_>>()?;
    let complete = rows.len() <= limit as usize;
    if !complete {
        rows.truncate(limit as usize);
    }
    let next_cursor = if complete {
        None
    } else {
        rows.last().map(|row| row.order_id)
    };
    Ok(OwnerInventoryResponse {
        schema_version: ORDER_API_SCHEMA_VERSION,
        snapshot,
        rows,
        next_cursor,
        complete,
    })
}

pub fn query_protocol(deps: Deps) -> StdResult<PairProtocolResponse> {
    Ok(PairProtocolResponse {
        schema_version: ORDER_API_SCHEMA_VERSION,
        features: vec![
            PairApiFeature::TypedOrderStatus,
            PairApiFeature::OwnerInventory,
            PairApiFeature::OwnerIndexBackfill,
        ],
        owner_inventory_ready: OWNER_INDEX_READY.load(deps.storage)?,
        owner_inventory_generation: OWNER_INDEX_GENERATION.load(deps.storage)?,
        max_owner_inventory_page_size: MAX_OWNER_INVENTORY_PAGE_SIZE,
    })
}

pub fn continue_backfill(storage: &mut dyn Storage, limit: u32) -> Result<Response, ContractError> {
    if limit == 0 || limit > MAX_OWNER_INDEX_BACKFILL_LIMIT {
        return Err(ContractError::OwnerIndexBackfillLimit {
            max: MAX_OWNER_INDEX_BACKFILL_LIMIT,
            actual: limit,
        });
    }
    if OWNER_INDEX_READY.load(storage)? {
        return Ok(Response::new()
            .add_attribute("action", "continue_owner_index_backfill")
            .add_attribute("processed", "0")
            .add_attribute("complete", "true"));
    }

    let mut cursor = OWNER_INDEX_BACKFILL_CURSOR.load(storage)?;
    if cursor.max_order_id.is_none() {
        cursor.max_order_id = Some(
            ORDER_NEXT_ID
                .load(storage)?
                .checked_sub(1)
                .ok_or_else(|| StdError::generic_err("invalid zero next order id"))?,
        );
    }
    let start = cursor.last_order_id.map(Bound::exclusive);
    let max_order_id = cursor
        .max_order_id
        .ok_or_else(|| StdError::generic_err("missing owner index backfill high-water"))?;
    let end = Some(Bound::inclusive(max_order_id));
    let processed;
    let phase_complete;
    match cursor.phase {
        OwnerIndexBackfillPhase::Active => {
            let rows: Vec<_> = ORDERS
                .range(storage, start, end, Order::Ascending)
                .take(limit as usize + 1)
                .collect::<StdResult<_>>()?;
            phase_complete = rows.len() <= limit as usize;
            processed = rows.len().min(limit as usize);
            for (order_id, order) in rows.into_iter().take(processed) {
                save_active(storage, order_id, &order)?;
                cursor.last_order_id = Some(order_id);
            }
            if phase_complete {
                cursor.phase = OwnerIndexBackfillPhase::ParkedRefund;
                cursor.last_order_id = None;
            }
        }
        OwnerIndexBackfillPhase::ParkedRefund => {
            let rows: Vec<_> = EXPIRED_LIMIT_CLAIMS
                .range(storage, start, end, Order::Ascending)
                .take(limit as usize + 1)
                .collect::<StdResult<_>>()?;
            phase_complete = rows.len() <= limit as usize;
            processed = rows.len().min(limit as usize);
            for (order_id, row) in rows.into_iter().take(processed) {
                save_parked(storage, order_id, &row)?;
                cursor.last_order_id = Some(order_id);
            }
            if phase_complete {
                OWNER_INDEX_READY.save(storage, &true)?;
                OWNER_INDEX_BACKFILL_CURSOR.remove(storage);
            }
        }
    }
    let complete = OWNER_INDEX_READY.load(storage)?;
    if !complete {
        OWNER_INDEX_BACKFILL_CURSOR.save(storage, &cursor)?;
    }
    Ok(Response::new()
        .add_attribute("action", "continue_owner_index_backfill")
        .add_attribute("processed", processed.to_string())
        .add_attribute("phase_complete", phase_complete.to_string())
        .add_attribute("complete", complete.to_string()))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use dex_common::pair::{LimitOrderSide, OrderStatus, OrderStatusReason, OwnerOrderState};

    use super::*;
    use crate::state::{OwnerIndexBackfillCursor, OWNER_ORDERS};

    fn active(owner: &str, remaining: u128) -> LimitOrder {
        LimitOrder {
            owner: Addr::unchecked(owner),
            price: Decimal::percent(125),
            remaining: Uint128::new(remaining),
            side: LimitOrderSide::Bid,
            expires_at: Some(1_000),
            prev: None,
            next: None,
        }
    }

    fn parked(owner: &str, remaining: u128) -> ExpiredLimitRefund {
        ExpiredLimitRefund {
            owner: Addr::unchecked(owner),
            side: LimitOrderSide::Bid,
            remaining: Uint128::new(remaining),
            expires_at: Some(1_000),
            price: Some(Decimal::percent(125)),
            reason: Some(OrderStatusReason::TimeExpired),
        }
    }

    fn ready(storage: &mut dyn Storage, next_id: u64) {
        OWNER_INDEX_READY.save(storage, &true).unwrap();
        OWNER_INDEX_GENERATION.save(storage, &7).unwrap();
        ORDER_NEXT_ID.save(storage, &next_id).unwrap();
    }

    #[test]
    fn typed_status_distinguishes_all_lifecycle_states() {
        let mut deps = mock_dependencies();
        let order = active("maker", 50);
        ORDERS.save(&mut deps.storage, 1, &order).unwrap();
        assert_eq!(
            query_status(deps.as_ref(), 1).unwrap().status,
            OrderStatus::Active
        );

        ORDERS.remove(&mut deps.storage, 1);
        let refund = parked("maker", 40);
        EXPIRED_LIMIT_CLAIMS
            .save(&mut deps.storage, 1, &refund)
            .unwrap();
        let status = query_status(deps.as_ref(), 1).unwrap();
        assert_eq!(status.status, OrderStatus::ParkedRefund);
        assert_eq!(status.owner, Some(Addr::unchecked("maker")));
        assert_eq!(status.price, Some(Decimal::percent(125)));

        EXPIRED_LIMIT_CLAIMS.remove(&mut deps.storage, 1);
        save_terminal(
            &mut deps.storage,
            1,
            &refund.owner,
            refund.side.clone(),
            refund.price,
            Uint128::zero(),
            refund.expires_at,
            OrderStatusReason::FullyExecuted,
            12,
            34,
        )
        .unwrap();
        let status = query_status(deps.as_ref(), 1).unwrap();
        assert_eq!(status.status, OrderStatus::FullyExecuted);
        assert_eq!(status.terminal_height, Some(12));
        assert_eq!(status.terminal_time, Some(34));

        save_terminal(
            &mut deps.storage,
            2,
            &refund.owner,
            refund.side,
            refund.price,
            refund.remaining,
            refund.expires_at,
            OrderStatusReason::Cancelled,
            56,
            78,
        )
        .unwrap();
        assert_eq!(
            query_status(deps.as_ref(), 2).unwrap().status,
            OrderStatus::Cancelled
        );
        assert_eq!(
            query_status(deps.as_ref(), 99).unwrap().status,
            OrderStatus::NotFound
        );
        assert!(query_status(deps.as_ref(), 0).is_err());
    }

    #[test]
    fn full_fill_path_writes_tombstone_and_removes_custody() {
        let mut deps = mock_dependencies();
        let maker = Addr::unchecked("maker");
        let order_id = crate::orderbook::insert_bid(
            &mut deps.storage,
            Decimal::one(),
            Uint128::new(100),
            maker.clone(),
            None,
            32,
            None,
        )
        .unwrap();
        crate::orderbook::match_bids_at_height(
            &mut deps.storage,
            500,
            42,
            Uint128::new(100),
            1,
            None,
            "pair",
            "token0",
            "token1",
            &Addr::unchecked("taker"),
            &Addr::unchecked("treasury"),
            0,
            None,
        )
        .unwrap();

        let status = query_status(deps.as_ref(), order_id).unwrap();
        assert_eq!(status.status, OrderStatus::FullyExecuted);
        assert_eq!(status.owner, Some(maker.clone()));
        assert_eq!(status.remaining, Some(Uint128::zero()));
        assert_eq!(status.terminal_height, Some(42));
        assert_eq!(status.terminal_time, Some(500));
        assert!(OWNER_ORDERS
            .may_load(&deps.storage, (&maker, order_id))
            .unwrap()
            .is_none());
    }

    #[test]
    fn pagination_is_owner_isolated_snapshot_bounded_and_transition_stable() {
        let mut deps = mock_dependencies();
        ready(&mut deps.storage, 6);
        for id in [1u64, 3, 5] {
            save_active(&mut deps.storage, id, &active("alice", id as u128)).unwrap();
        }
        save_active(&mut deps.storage, 2, &active("bob", 2)).unwrap();

        let first =
            query_owner_inventory(deps.as_ref(), "alice".into(), None, None, Some(2)).unwrap();
        assert_eq!(
            first
                .rows
                .iter()
                .map(|row| row.order_id)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(first.next_cursor, Some(3));
        assert!(!first.complete);

        save_active(&mut deps.storage, 6, &active("alice", 6)).unwrap();
        ORDER_NEXT_ID.save(&mut deps.storage, &7).unwrap();
        let refund = parked("alice", 5);
        save_parked(&mut deps.storage, 5, &refund).unwrap();

        let second = query_owner_inventory(
            deps.as_ref(),
            "alice".into(),
            Some(first.snapshot),
            first.next_cursor,
            Some(2),
        )
        .unwrap();
        assert_eq!(second.rows.len(), 1);
        assert_eq!(second.rows[0].order_id, 5);
        assert_eq!(second.rows[0].state, OwnerOrderState::ParkedRefund);
        assert!(second.complete);
        assert_eq!(second.next_cursor, None);
    }

    #[test]
    fn inventory_fails_closed_when_not_ready_or_generation_changes() {
        let mut deps = mock_dependencies();
        ORDER_NEXT_ID.save(&mut deps.storage, &4).unwrap();
        OWNER_INDEX_READY.save(&mut deps.storage, &false).unwrap();
        OWNER_INDEX_GENERATION.save(&mut deps.storage, &1).unwrap();
        assert!(query_owner_inventory(deps.as_ref(), "alice".into(), None, None, None).is_err());

        OWNER_INDEX_READY.save(&mut deps.storage, &true).unwrap();
        assert!(query_owner_inventory(
            deps.as_ref(),
            "alice".into(),
            Some(OwnerInventorySnapshot {
                generation: 2,
                max_order_id: 0,
            }),
            None,
            None,
        )
        .is_err());

        for future_max in [4, u64::MAX] {
            assert!(query_owner_inventory(
                deps.as_ref(),
                "alice".into(),
                Some(OwnerInventorySnapshot {
                    generation: 1,
                    max_order_id: future_max,
                }),
                None,
                None,
            )
            .is_err());
        }

        let old_snapshot = OwnerInventorySnapshot {
            generation: 1,
            max_order_id: 2,
        };
        ORDER_NEXT_ID.save(&mut deps.storage, &10).unwrap();
        let response = query_owner_inventory(
            deps.as_ref(),
            "alice".into(),
            Some(old_snapshot.clone()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(response.snapshot, old_snapshot);
    }

    #[test]
    fn backfill_is_resumable_idempotent_and_transition_safe() {
        let mut deps = mock_dependencies();
        OWNER_INDEX_READY.save(&mut deps.storage, &false).unwrap();
        OWNER_INDEX_GENERATION.save(&mut deps.storage, &1).unwrap();
        OWNER_INDEX_BACKFILL_CURSOR
            .save(
                &mut deps.storage,
                &OwnerIndexBackfillCursor {
                    phase: OwnerIndexBackfillPhase::Active,
                    last_order_id: None,
                    max_order_id: Some(3),
                },
            )
            .unwrap();
        ORDERS
            .save(&mut deps.storage, 1, &active("alice", 10))
            .unwrap();
        ORDERS
            .save(&mut deps.storage, 3, &active("alice", 30))
            .unwrap();
        EXPIRED_LIMIT_CLAIMS
            .save(&mut deps.storage, 2, &parked("bob", 20))
            .unwrap();

        continue_backfill(&mut deps.storage, 1).unwrap();
        ORDERS.remove(&mut deps.storage, 3);
        let transitioned = parked("alice", 30);
        EXPIRED_LIMIT_CLAIMS
            .save(&mut deps.storage, 3, &transitioned)
            .unwrap();
        save_parked(&mut deps.storage, 3, &transitioned).unwrap();

        while !OWNER_INDEX_READY.load(&deps.storage).unwrap() {
            continue_backfill(&mut deps.storage, 1).unwrap();
        }
        let alice: Vec<_> = OWNER_ORDERS
            .prefix(&Addr::unchecked("alice"))
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<_>>()
            .unwrap();
        assert_eq!(alice.len(), 2);
        assert_eq!(alice[0].0, 1);
        assert_eq!(alice[1].0, 3);
        assert_eq!(alice[1].1.state, OwnerOrderState::ParkedRefund);

        let response = continue_backfill(&mut deps.storage, 1).unwrap();
        assert!(response
            .attributes
            .iter()
            .any(|attr| { attr.key == "processed" && attr.value == "0" }));
    }

    #[test]
    fn backfill_high_water_excludes_continual_new_ids_but_keeps_dual_writes() {
        let mut deps = mock_dependencies();
        OWNER_INDEX_READY.save(&mut deps.storage, &false).unwrap();
        OWNER_INDEX_GENERATION.save(&mut deps.storage, &1).unwrap();
        OWNER_INDEX_BACKFILL_CURSOR
            .save(
                &mut deps.storage,
                &OwnerIndexBackfillCursor {
                    phase: OwnerIndexBackfillPhase::Active,
                    last_order_id: None,
                    max_order_id: Some(2),
                },
            )
            .unwrap();
        ORDERS
            .save(&mut deps.storage, 1, &active("legacy-active", 10))
            .unwrap();
        EXPIRED_LIMIT_CLAIMS
            .save(&mut deps.storage, 2, &parked("legacy-parked", 20))
            .unwrap();

        let mut processed = 0u32;
        let mut next_id = 3u64;
        while !OWNER_INDEX_READY.load(&deps.storage).unwrap() {
            let new_order = active("new-owner", next_id as u128);
            ORDERS.save(&mut deps.storage, next_id, &new_order).unwrap();
            save_active(&mut deps.storage, next_id, &new_order).unwrap();
            ORDER_NEXT_ID
                .save(&mut deps.storage, &next_id.checked_add(1).unwrap())
                .unwrap();

            let response = continue_backfill(&mut deps.storage, 1).unwrap();
            processed += response
                .attributes
                .iter()
                .find(|attr| attr.key == "processed")
                .unwrap()
                .value
                .parse::<u32>()
                .unwrap();
            next_id = next_id.checked_add(1).unwrap();
            assert!(next_id < 10, "backfill did not complete at its high-water");
        }

        assert_eq!(processed, 2, "only migration-time rows are scanned");
        for id in 3..next_id {
            assert!(OWNER_ORDERS
                .may_load(&deps.storage, (&Addr::unchecked("new-owner"), id))
                .unwrap()
                .is_some());
        }
    }
}
