/**
 * Retail execute-msg gas inventory (GitLab #475).
 *
 * Static envelopes in {@link getGasLimitForTx} are the canonical fee path — there is no LCD
 * simulate-before-broadcast. Every retail `executeTerraContract` / `executeTerraContractMulti`
 * shape (and CW20 `send` inner) must be listed here so CI fails when a new shape falls through
 * to {@link BASE_GAS_LIMIT} unnoticed (same failure class as #384 register / #474 drip).
 *
 * **Invariant G-RETAIL-1:** For every fixture in {@link RETAIL_GAS_SHAPE_FIXTURES}, either:
 * - the top-level key is in {@link BASE_GAS_LIMIT_ALLOWLIST} and limit === BASE_GAS_LIMIT, or
 * - `getGasLimitForTx(msg) > BASE_GAS_LIMIT`.
 *
 * **Invariant G-RETAIL-2:** Intentional BASE uses are only CW20 allowance adjust msgs
 * (`increase_allowance` / `decrease_allowance`). Do not add economic executes to the allowlist.
 *
 * When adding a retail execute: named constant + `getGasLimitForTx` branch (+ send-inner if
 * CW20 hook) + fixture here + docs row in `docs/frontend.md` § Terra Classic gas limits.
 * Playbook: `skills/AGENTS_TERRACLASSIC_GAS.md`.
 */

import { UNWRAP_GAS_LIMIT, UST1_WINDOW_SEND_GAS_LIMIT, WRAP_GAS_LIMIT } from '@/utils/constants'
import {
  ADD_LIQUIDITY_GAS_LIMIT,
  BASE_GAS_LIMIT,
  CANCEL_LIMIT_ORDER_GAS_LIMIT,
  CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT,
  CREATE_PAIR_GAS_LIMIT,
  DEREGISTER_FEE_DISCOUNT_GAS_LIMIT,
  FAUCET_DRIP_GAS_LIMIT,
  REGISTER_FEE_DISCOUNT_GAS_LIMIT,
  REMOVE_LIQUIDITY_GAS_LIMIT,
  UPDATE_LIMIT_ORDER_PRICE_GAS_LIMIT,
  gasLimitForExecuteSwapOperations,
  gasLimitForLimitOrderBatch,
  gasLimitForLimitOrderCancelBatch,
  gasLimitForRouterExecuteSwapOperations,
} from './terraGas'

export type RetailGasShapeFixture = {
  /** Stable id for CI / docs cross-refs. */
  id: string
  /** Human note: call site / feature. */
  note: string
  msg: Record<string, unknown>
  /** Exact expected gas limit from `getGasLimitForTx`. */
  expectedGas: number
}

function b64(inner: Record<string, unknown>): string {
  return btoa(JSON.stringify(inner))
}

/**
 * Complete retail shape table for unit-test guardrail.
 * Keep in lockstep with `getGasLimitForTx` branches in `terraGas.ts`.
 */
export const RETAIL_GAS_SHAPE_FIXTURES: readonly RetailGasShapeFixture[] = [
  {
    id: 'wrap_deposit',
    note: 'router/wrapMapper native wrap',
    msg: { wrap_deposit: {} },
    expectedGas: WRAP_GAS_LIMIT,
  },
  {
    id: 'drip',
    note: 'Mint faucet soft-launch (#474 / #475)',
    msg: { drip: { token: 'terra1token' } },
    expectedGas: FAUCET_DRIP_GAS_LIMIT,
  },
  {
    id: 'create_pair',
    note: 'CreatePairPage (#345)',
    msg: { create_pair: { asset_infos: [] } },
    expectedGas: CREATE_PAIR_GAS_LIMIT,
  },
  {
    id: 'register',
    note: '/tiers register (#384)',
    msg: { register: { tier_id: 1 } },
    expectedGas: REGISTER_FEE_DISCOUNT_GAS_LIMIT,
  },
  {
    id: 'deregister',
    note: '/tiers deregister (#384)',
    msg: { deregister: {} },
    expectedGas: DEREGISTER_FEE_DISCOUNT_GAS_LIMIT,
  },
  {
    id: 'provide_liquidity',
    note: 'pair provideLiquidity',
    msg: { provide_liquidity: {} },
    expectedGas: ADD_LIQUIDITY_GAS_LIMIT,
  },
  {
    id: 'withdraw_liquidity',
    note: 'top-level withdraw (rare; send-inner is retail)',
    msg: { withdraw_liquidity: {} },
    expectedGas: REMOVE_LIQUIDITY_GAS_LIMIT,
  },
  {
    id: 'swap_pool_only',
    note: 'direct pair swap (pool-only)',
    msg: { swap: {} },
    expectedGas: gasLimitForExecuteSwapOperations(1),
  },
  {
    id: 'execute_swap_operations_1hop',
    note: 'router single-hop (#353)',
    msg: { execute_swap_operations: { operations: [{ terra_swap: {} }] } },
    expectedGas: gasLimitForRouterExecuteSwapOperations(1),
  },
  {
    id: 'cancel_limit_order',
    note: 'pair cancelLimitOrder',
    msg: { cancel_limit_order: { order_id: 1 } },
    expectedGas: CANCEL_LIMIT_ORDER_GAS_LIMIT,
  },
  {
    id: 'cancel_limit_orders',
    note: 'pair cancelLimitOrders batch (#246)',
    msg: { cancel_limit_orders: { order_ids: [1, 2] } },
    expectedGas: gasLimitForLimitOrderCancelBatch(2),
  },
  {
    id: 'claim_expired_limit_order',
    note: 'pair claimExpiredLimitOrder',
    msg: { claim_expired_limit_order: { order_id: 1 } },
    expectedGas: CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT,
  },
  {
    id: 'claim_expired_limit_orders',
    note: 'pair claimExpiredLimitOrders batch (#246)',
    msg: { claim_expired_limit_orders: { order_ids: [1, 2, 3] } },
    expectedGas: gasLimitForLimitOrderCancelBatch(3),
  },
  {
    id: 'update_limit_order_price',
    note: 'pair updateLimitOrderPrice (#247)',
    msg: { update_limit_order_price: { order_id: 1, new_price: '1' } },
    expectedGas: UPDATE_LIMIT_ORDER_PRICE_GAS_LIMIT,
  },
  {
    id: 'place_limit_order_batch',
    note: 'top-level batch (hooks / tests); retail uses send-inner',
    msg: {
      place_limit_order_batch: {
        side: 'bid',
        orders: [{ price: '1', amount: '100', max_adjust_steps: 32 }],
      },
    },
    expectedGas: gasLimitForLimitOrderBatch(1),
  },
  {
    id: 'place_limit_order_ladder',
    note: 'top-level ladder; retail uses send-inner',
    msg: {
      place_limit_order_ladder: {
        ladder: { count: 3 },
      },
    },
    expectedGas: gasLimitForLimitOrderBatch(3),
  },
  {
    id: 'increase_allowance',
    note: 'intentional BASE (#127 / #132 / #147)',
    msg: { increase_allowance: { spender: 'terra1pair', amount: '1' } },
    expectedGas: BASE_GAS_LIMIT,
  },
  {
    id: 'decrease_allowance',
    note: 'intentional BASE (provide-liquidity rollback)',
    msg: { decrease_allowance: { spender: 'terra1pair', amount: '1' } },
    expectedGas: BASE_GAS_LIMIT,
  },
  {
    id: 'send_inner_swap',
    note: 'pair.swap / market hop',
    msg: { send: { msg: b64({ swap: {} }) } },
    expectedGas: gasLimitForExecuteSwapOperations(1),
  },
  {
    id: 'send_inner_unwrap',
    note: 'router/wrapMapper / PoolPage unwrap (#343 / #475)',
    msg: { send: { msg: b64({ unwrap: { recipient: null } }) } },
    expectedGas: UNWRAP_GAS_LIMIT,
  },
  {
    id: 'send_inner_ust1_deposit',
    note: 'ust1-window CW20 Send deposit (#506)',
    msg: { send: { msg: b64({ deposit: {} }) } },
    expectedGas: UST1_WINDOW_SEND_GAS_LIMIT,
  },
  {
    id: 'send_inner_ust1_withdraw',
    note: 'ust1-window CW20 Send withdraw (#506)',
    msg: { send: { msg: b64({ withdraw: { min_vfdusd_out: '1' } }) } },
    expectedGas: UST1_WINDOW_SEND_GAS_LIMIT,
  },
  {
    id: 'send_inner_withdraw_liquidity',
    note: 'pair withdrawLiquidity',
    msg: { send: { msg: b64({ withdraw_liquidity: {} }) } },
    expectedGas: REMOVE_LIQUIDITY_GAS_LIMIT,
  },
  {
    id: 'send_inner_execute_swap_operations',
    note: 'router multi-hop CW20 path',
    msg: {
      send: {
        msg: b64({
          execute_swap_operations: { operations: [{ terra_swap: {} }, { terra_swap: {} }] },
        }),
      },
    },
    expectedGas: gasLimitForRouterExecuteSwapOperations(2),
  },
  {
    id: 'send_inner_place_limit_order_batch',
    note: 'placeLimitOrderBatchWithAllowance',
    msg: {
      send: {
        msg: b64({
          place_limit_order_batch: {
            side: 'bid',
            orders: [{ price: '1', amount: '100', max_adjust_steps: 32 }],
          },
        }),
      },
    },
    expectedGas: gasLimitForLimitOrderBatch(1),
  },
  {
    id: 'send_inner_place_limit_order_ladder',
    note: 'placeLimitOrderLadderWithAllowance',
    msg: {
      send: {
        msg: b64({
          place_limit_order_ladder: {
            ladder: { count: 4 },
          },
        }),
      },
    },
    expectedGas: gasLimitForLimitOrderBatch(4),
  },
]
