import { CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'

const BASE_GAS_LIMIT = 200_000
const SWAP_GAS_LIMIT = 600_000
const HYBRID_SWAP_GAS_LIMIT = 1_200_000
const PLACE_LIMIT_ORDER_GAS_LIMIT = 950_000
const ADD_LIQUIDITY_GAS_LIMIT = 500_000
const REMOVE_LIQUIDITY_GAS_LIMIT = 600_000
const SWAP_GAS_BUFFER = 1.2
const SWAP_MULTIHOP_GAS_PADDING_PER_HOP = 50_000
const EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP = 661_000

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
  return Math.max(padded, floor)
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
    return innerSwapUsesHybrid(executeMsg as Record<string, unknown>) ? HYBRID_SWAP_GAS_LIMIT : SWAP_GAS_LIMIT
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
        if ('place_limit_order' in inner) return PLACE_LIMIT_ORDER_GAS_LIMIT
        if ('swap' in inner) {
          return innerSwapUsesHybrid(inner) ? HYBRID_SWAP_GAS_LIMIT : SWAP_GAS_LIMIT
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
