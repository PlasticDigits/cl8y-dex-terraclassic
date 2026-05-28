import { CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'

const BASE_GAS_LIMIT = 200_000
const HYBRID_SWAP_GAS_LIMIT = 1_200_000
const PLACE_LIMIT_ORDER_GAS_LIMIT = 950_000
const PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT = 400_000
const PLACE_LIMIT_ORDER_BATCH_PER_RUNG_GAS_LIMIT = 180_000
const ADD_LIQUIDITY_GAS_LIMIT = 500_000
const REMOVE_LIQUIDITY_GAS_LIMIT = 600_000
/** Keep in sync with `SWAP_GAS_BUFFER` in `frontend-dapp/src/utils/constants.ts` (GitLab #115). */
const SWAP_GAS_BUFFER = 1.3
const SWAP_MULTIHOP_GAS_PADDING_PER_HOP = 50_000
const EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP = 661_000
/** Keep in sync with `SWAP_GAS_SAFETY_MARGIN` in `frontend-dapp/src/utils/constants.ts`. */
const SWAP_GAS_SAFETY_MARGIN = 10_000

function countSwapHops(msg: Record<string, unknown>): number {
  const ops = (msg as { execute_swap_operations?: { operations?: unknown[] } }).execute_swap_operations
  return ops?.operations?.length ?? 1
}

function innerSwapUsesHybrid(inner: Record<string, unknown>): boolean {
  const sw = inner.swap as { hybrid?: unknown } | undefined
  return !!(sw && sw.hybrid != null)
}

function executeSwapOpsUsesHybrid(inner: Record<string, unknown>): boolean {
  const e = inner.execute_swap_operations as
    | { operations?: Array<{ terra_swap?: { hybrid?: unknown } }> }
    | undefined
  if (!e?.operations) return false
  return e.operations.some((op) => op.terra_swap?.hybrid != null)
}

function gasLimitForExecuteSwapOperations(hops: number): number {
  const hopCount = Math.max(hops, 1)
  const scaled = Math.round(600_000 * hopCount * SWAP_GAS_BUFFER)
  const padded = scaled + hopCount * SWAP_MULTIHOP_GAS_PADDING_PER_HOP
  const floor = hopCount * EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP
  return Math.max(padded, floor) + SWAP_GAS_SAFETY_MARGIN
}

function gasLimitForSwapOperationsMsg(msg: Record<string, unknown>): number {
  const hops = countSwapHops(msg)
  const poolOnly = gasLimitForExecuteSwapOperations(hops)
  if (executeSwapOpsUsesHybrid(msg)) {
    return Math.max(poolOnly, HYBRID_SWAP_GAS_LIMIT * hops)
  }
  return poolOnly
}

export function getGasLimitForExecuteMsg(executeMsg: Record<string, unknown>): number {
  if ('execute_swap_operations' in executeMsg) {
    return gasLimitForSwapOperationsMsg(executeMsg)
  }
  if ('swap' in executeMsg) {
    return innerSwapUsesHybrid(executeMsg as Record<string, unknown>)
      ? HYBRID_SWAP_GAS_LIMIT
      : gasLimitForExecuteSwapOperations(1)
  }
  if ('provide_liquidity' in executeMsg) {
    return ADD_LIQUIDITY_GAS_LIMIT
  }
  if ('withdraw_liquidity' in executeMsg) {
    return REMOVE_LIQUIDITY_GAS_LIMIT
  }
  if ('send' in executeMsg) {
    const sendMsg = executeMsg.send as { msg?: string } | undefined
    if (sendMsg?.msg) {
      try {
        const inner = JSON.parse(Buffer.from(sendMsg.msg, 'base64').toString('utf8')) as Record<string, unknown>
        if ('place_limit_order_batch' in inner || 'place_limit_order_ladder' in inner) {
          const batch = inner.place_limit_order_batch as { orders?: unknown[] } | undefined
          const ladder = inner.place_limit_order_ladder as { ladder?: { count?: number } } | undefined
          const n = batch?.orders?.length ?? ladder?.ladder?.count ?? 1
          return PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT + PLACE_LIMIT_ORDER_BATCH_PER_RUNG_GAS_LIMIT * n
        }
        if ('swap' in inner) {
          return innerSwapUsesHybrid(inner) ? HYBRID_SWAP_GAS_LIMIT : gasLimitForExecuteSwapOperations(1)
        }
        if ('withdraw_liquidity' in inner) return REMOVE_LIQUIDITY_GAS_LIMIT
        if ('execute_swap_operations' in inner) return gasLimitForSwapOperationsMsg(inner)
      } catch {
        /* fall through */
      }
    }
  }
  if ('increase_allowance' in executeMsg || 'decrease_allowance' in executeMsg) {
    return BASE_GAS_LIMIT
  }
  return BASE_GAS_LIMIT
}

export function estimateTerraClassicFee(gasLimit: number, gasPriceUluna: string): Fee {
  const feeAmount = Math.ceil(parseFloat(gasPriceUluna) * gasLimit)
  return new Fee({
    amount: [{ amount: feeAmount.toString(), denom: 'uluna' }],
    gasLimit: BigInt(gasLimit),
  })
}
