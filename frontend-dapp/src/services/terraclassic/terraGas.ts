import { CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'
import {
  EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP,
  SWAP_GAS_BUFFER,
  SWAP_GAS_PER_HOP,
  SWAP_MULTIHOP_GAS_PADDING_PER_HOP,
  WRAP_GAS_LIMIT,
  effectiveGasPriceUluna,
} from '@/utils/constants'

export const BASE_GAS_LIMIT = 200000
export const SWAP_GAS_LIMIT = 600000
/** Pattern C / limit-book matching uses more gas than pool-only swaps. */
export const HYBRID_SWAP_GAS_LIMIT = 1200000
export const PLACE_LIMIT_ORDER_GAS_LIMIT = 950000
export const CANCEL_LIMIT_ORDER_GAS_LIMIT = 450000
export const CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT = 450000
export const ADD_LIQUIDITY_GAS_LIMIT = 500000
export const REMOVE_LIQUIDITY_GAS_LIMIT = 600000
export const CREATE_PAIR_GAS_LIMIT = 800000

/** Uluna in `Fee.amount` for one message at `gasLimit` (same math as {@link buildTerraClassicFee}). */
export function estimateFeeUlunaAmountForGasLimit(gasLimit: number): bigint {
  return BigInt(Math.ceil(effectiveGasPriceUluna() * gasLimit))
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

function gasLimitForSwapOperationsMsg(msg: Record<string, unknown>): number {
  const hops = countSwapHops(msg)
  const poolOnly = gasLimitForExecuteSwapOperations(hops)
  if (executeSwapOpsUsesHybrid(msg)) {
    return Math.max(poolOnly, HYBRID_SWAP_GAS_LIMIT * hops)
  }
  return poolOnly
}

/** Buffered estimate + per-hop padding, floored at min gas per hop (see constants). */
function gasLimitForExecuteSwapOperations(hops: number): number {
  const hopCount = Math.max(hops, 1)
  const scaled = Math.round(SWAP_GAS_PER_HOP * hopCount * SWAP_GAS_BUFFER)
  const padded = scaled + hopCount * SWAP_MULTIHOP_GAS_PADDING_PER_HOP
  const floor = hopCount * EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP
  return Math.max(padded, floor)
}

/**
 * Gas limit for a single `MsgExecuteContract` execute payload.
 * Shared by broadcast fee math and UI preflight sequence estimates (GitLab #127).
 */
export function getGasLimitForTx(executeMsg: Record<string, unknown>): number {
  if ('wrap_deposit' in executeMsg) {
    return WRAP_GAS_LIMIT
  }
  if ('place_limit_order' in executeMsg) {
    return PLACE_LIMIT_ORDER_GAS_LIMIT
  }
  if ('cancel_limit_order' in executeMsg) {
    return CANCEL_LIMIT_ORDER_GAS_LIMIT
  }
  if ('claim_expired_limit_order' in executeMsg) {
    return CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT
  }
  if ('execute_swap_operations' in executeMsg) {
    return gasLimitForSwapOperationsMsg(executeMsg)
  } else if ('swap' in executeMsg) {
    return SWAP_GAS_LIMIT
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
        if ('place_limit_order' in inner) return PLACE_LIMIT_ORDER_GAS_LIMIT
        if ('swap' in inner) {
          return innerSwapUsesHybrid(inner) ? HYBRID_SWAP_GAS_LIMIT : SWAP_GAS_LIMIT
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
