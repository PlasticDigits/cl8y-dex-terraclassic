import { CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'
import {
  EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP,
  SWAP_GAS_BUFFER,
  SWAP_GAS_PER_HOP,
  SWAP_GAS_SAFETY_MARGIN,
  SWAP_MULTIHOP_GAS_PADDING_PER_HOP,
  WRAP_GAS_LIMIT,
  effectiveGasPriceUluna,
} from '@/utils/constants'
import { HYBRID_SWAP_GAS_LIMIT, gasLimitForHybridParams, hybridSwapParamsFromRecord } from './hybridSwapGas'

export { HYBRID_SWAP_GAS_LIMIT }
export { gasLimitForHybridSwap, gasLimitForHybridParams } from './hybridSwapGas'

export const BASE_GAS_LIMIT = 200000
/** Legacy per-hop base; pool-only broadcast uses {@link gasLimitForExecuteSwapOperations}(1) (840k, GitLab #115 / #134). */
export const SWAP_GAS_LIMIT = 600000
/** Pattern C / limit-book matching — flat fallback when quote-driven estimate unavailable (GitLab #249). */
export const PLACE_LIMIT_ORDER_GAS_LIMIT = 950000
/** Base gas for one CW20 send → `place_limit_order_batch` / ladder (GitLab #206). */
export const PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT = 400000
/** Per-rung marginal gas on top of batch base. */
export const PLACE_LIMIT_ORDER_BATCH_PER_RUNG_GAS_LIMIT = 180000
export const CANCEL_LIMIT_ORDER_GAS_LIMIT = 450000
/** Base gas for one `cancel_limit_orders` / `claim_expired_limit_orders` batch tx (GitLab #246). */
export const CANCEL_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT = 400000
/** Per-order marginal gas on top of batch cancel/claim base. */
export const CANCEL_LIMIT_ORDER_BATCH_PER_ORDER_GAS_LIMIT = 80000
/** In-place limit price relink — one pair execute, no CW20 (GitLab #247; ≪ cancel+place). */
export const UPDATE_LIMIT_ORDER_PRICE_GAS_LIMIT = 350000
export const CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT = 450000
export const ADD_LIQUIDITY_GAS_LIMIT = 650000
export const REMOVE_LIQUIDITY_GAS_LIMIT = 600000
export const CREATE_PAIR_GAS_LIMIT = 800000

/** Uluna in `Fee.amount` for one message at `gasLimit` (same math as {@link buildTerraClassicFee}). */
export function estimateFeeUlunaAmountForGasLimit(gasLimit: number): bigint {
  return BigInt(Math.ceil(effectiveGasPriceUluna() * gasLimit))
}

export function gasLimitForLimitOrderBatch(rungCount: number): number {
  const n = Math.max(1, Math.floor(rungCount))
  return PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT + PLACE_LIMIT_ORDER_BATCH_PER_RUNG_GAS_LIMIT * n
}

export function gasLimitForLimitOrderCancelBatch(orderCount: number): number {
  const n = Math.max(1, Math.floor(orderCount))
  return CANCEL_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT + CANCEL_LIMIT_ORDER_BATCH_PER_ORDER_GAS_LIMIT * n
}

export function buildTerraClassicFee(gasLimit: number): Fee {
  const feeAmount = Number(estimateFeeUlunaAmountForGasLimit(gasLimit))

  return new Fee({
    amount: [
      {
        amount: feeAmount.toString(),
        denom: 'uluna',
      },
    ],
    gasLimit: BigInt(gasLimit),
  })
}

function countSwapHops(msg: Record<string, unknown>): number {
  const ops = (msg as { execute_swap_operations?: { operations?: unknown[] } }).execute_swap_operations
  return ops?.operations?.length ?? 1
}

function innerSwapUsesHybrid(inner: Record<string, unknown>): boolean {
  const sw = inner.swap as { hybrid?: unknown } | undefined
  return !!(sw && sw.hybrid != null)
}

function executeSwapOpsUsesHybrid(msg: Record<string, unknown>): boolean {
  const e = msg.execute_swap_operations as { operations?: Array<{ terra_swap?: { hybrid?: unknown } }> } | undefined
  if (!e?.operations) return false
  return e.operations.some((op) => op.terra_swap?.hybrid != null)
}

function gasLimitForHybridRouterOperations(msg: Record<string, unknown>): number {
  const e = msg.execute_swap_operations as { operations?: Array<{ terra_swap?: { hybrid?: unknown } }> } | undefined
  if (!e?.operations?.length) return HYBRID_SWAP_GAS_LIMIT
  let total = 0
  for (const op of e.operations) {
    const hybrid = hybridSwapParamsFromRecord(op.terra_swap?.hybrid as Record<string, unknown> | undefined)
    total += gasLimitForHybridParams(hybrid)
  }
  return total
}

function gasLimitForSwapOperationsMsg(msg: Record<string, unknown>): number {
  const hops = countSwapHops(msg)
  const poolOnly = gasLimitForExecuteSwapOperations(hops)
  if (executeSwapOpsUsesHybrid(msg)) {
    return Math.max(poolOnly, gasLimitForHybridRouterOperations(msg))
  }
  return poolOnly
}

/** Buffered estimate + per-hop padding, floored at min gas per hop (see constants). */
export function gasLimitForExecuteSwapOperations(hops: number): number {
  const hopCount = Math.max(hops, 1)
  const scaled = Math.round(SWAP_GAS_PER_HOP * hopCount * SWAP_GAS_BUFFER)
  const padded = scaled + hopCount * SWAP_MULTIHOP_GAS_PADDING_PER_HOP
  const floor = hopCount * EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP
  return Math.max(padded, floor) + SWAP_GAS_SAFETY_MARGIN
}

/**
 * Gas limit for a single `MsgExecuteContract` execute payload.
 * Shared by broadcast fee math and UI preflight sequence estimates (GitLab #127).
 */
export function getGasLimitForTx(executeMsg: Record<string, unknown>): number {
  if ('wrap_deposit' in executeMsg) {
    return WRAP_GAS_LIMIT
  }
  if ('place_limit_order_batch' in executeMsg || 'place_limit_order_ladder' in executeMsg) {
    const batch = executeMsg.place_limit_order_batch as { orders?: unknown[] } | undefined
    const ladder = executeMsg.place_limit_order_ladder as { ladder?: { count?: number } } | undefined
    const n = batch?.orders?.length ?? ladder?.ladder?.count ?? 1
    return gasLimitForLimitOrderBatch(n)
  }
  if ('cancel_limit_order' in executeMsg) {
    return CANCEL_LIMIT_ORDER_GAS_LIMIT
  }
  if ('update_limit_order_price' in executeMsg) {
    return UPDATE_LIMIT_ORDER_PRICE_GAS_LIMIT
  }
  if ('cancel_limit_orders' in executeMsg) {
    const batch = executeMsg.cancel_limit_orders as { order_ids?: unknown[] }
    return gasLimitForLimitOrderCancelBatch(batch.order_ids?.length ?? 1)
  }
  if ('claim_expired_limit_order' in executeMsg) {
    return CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT
  }
  if ('claim_expired_limit_orders' in executeMsg) {
    const batch = executeMsg.claim_expired_limit_orders as { order_ids?: unknown[] }
    return gasLimitForLimitOrderCancelBatch(batch.order_ids?.length ?? 1)
  }
  if ('execute_swap_operations' in executeMsg) {
    return gasLimitForSwapOperationsMsg(executeMsg)
  } else if ('swap' in executeMsg) {
    if (innerSwapUsesHybrid(executeMsg)) {
      const hybrid = hybridSwapParamsFromRecord((executeMsg.swap as { hybrid?: Record<string, unknown> }).hybrid)
      return gasLimitForHybridParams(hybrid)
    }
    return gasLimitForExecuteSwapOperations(1)
  } else if ('provide_liquidity' in executeMsg) {
    return ADD_LIQUIDITY_GAS_LIMIT
  } else if ('withdraw_liquidity' in executeMsg) {
    return REMOVE_LIQUIDITY_GAS_LIMIT
  } else if ('create_pair' in executeMsg) {
    return CREATE_PAIR_GAS_LIMIT
  } else if ('send' in executeMsg) {
    const sendMsg = executeMsg.send as { msg?: string } | undefined
    if (sendMsg?.msg) {
      try {
        const inner = JSON.parse(atob(sendMsg.msg)) as Record<string, unknown>
        if ('place_limit_order_batch' in inner) {
          const batch = inner.place_limit_order_batch as { orders?: unknown[] }
          return gasLimitForLimitOrderBatch(batch.orders?.length ?? 1)
        }
        if ('place_limit_order_ladder' in inner) {
          const ladder = inner.place_limit_order_ladder as { ladder?: { count?: number } }
          return gasLimitForLimitOrderBatch(ladder.ladder?.count ?? 1)
        }
        if ('swap' in inner) {
          if (innerSwapUsesHybrid(inner)) {
            const hybrid = hybridSwapParamsFromRecord((inner.swap as { hybrid?: Record<string, unknown> }).hybrid)
            return gasLimitForHybridParams(hybrid)
          }
          return gasLimitForExecuteSwapOperations(1)
        }
        if ('withdraw_liquidity' in inner) return REMOVE_LIQUIDITY_GAS_LIMIT
        if ('execute_swap_operations' in inner) return gasLimitForSwapOperationsMsg(inner)
      } catch {
        // fall through to base
      }
    }
    return SWAP_GAS_LIMIT
  } else if ('increase_allowance' in executeMsg || 'decrease_allowance' in executeMsg) {
    return BASE_GAS_LIMIT
  }
  return BASE_GAS_LIMIT
}

export function totalGasLimitForExecuteMsgs(messages: Array<{ msg: Record<string, unknown> }>): number {
  return messages.reduce((sum, m) => sum + getGasLimitForTx(m.msg), 0)
}
