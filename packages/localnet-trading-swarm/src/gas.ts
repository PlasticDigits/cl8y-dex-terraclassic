import { CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'

const BASE_GAS_LIMIT = 200_000
/** Keep in sync with `frontend-dapp/src/services/terraclassic/hybridSwapGas.ts` (GitLab #262). */
const HYBRID_SWAP_GAS_LIMIT = 15_000_000
/** Keep in sync with `frontend-dapp/src/utils/constants.ts` (GitLab #249, #260). */
const HYBRID_SWAP_BASE_GAS = 550_000
const HYBRID_SWAP_PER_MAKER_GAS = 65_000
const HYBRID_SWAP_MAKER_GAS_BUFFER = 2
const HYBRID_SWAP_GAS_FLOOR = 600_000
/** Keep in sync with `frontend-dapp/src/utils/constants.ts` (GitLab #260). */
const HYBRID_SWAP_PER_SCAN_STEP_GAS = 950
const HYBRID_SWAP_PER_EXPIRED_PARK_GAS = 8_000
/** Keep in sync with `frontend-dapp/src/services/terraclassic/hybridBookWalkLimits.ts` (GitLab #262). */
const MAX_SCAN_STEPS = 500
const MAX_EXPIRED_PARKS_PER_SWAP = 15
/** Keep in sync with `frontend-dapp/src/services/terraclassic/terraGas.ts` (#625 tax place). */
const PLACE_LIMIT_ORDER_GAS_LIMIT = 1_200_000
const PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT = 1_000_000
const PLACE_LIMIT_ORDER_BATCH_PER_RUNG_GAS_LIMIT = 180_000
/** Keep in sync with `frontend-dapp/src/services/terraclassic/terraGas.ts` (#625 tax provide). */
const ADD_LIQUIDITY_GAS_LIMIT = 1_000_000
const REMOVE_LIQUIDITY_GAS_LIMIT = 900_000
/** Keep in sync with `SWAP_GAS_BUFFER` in `frontend-dapp/src/utils/constants.ts` (GitLab #115). */
const SWAP_GAS_BUFFER = 1.3
const SWAP_MULTIHOP_GAS_PADDING_PER_HOP = 50_000
const EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP = 661_000
/** Keep in sync with `ROUTER_SINGLE_HOP_GAS_LIMIT` in `frontend-dapp/src/utils/constants.ts` (#353). */
const ROUTER_SINGLE_HOP_GAS_LIMIT = 1_400_000
/**
 * Keep in sync with `ROUTER_SWAP_OPS_MIN_GAS_PER_HOP` (#353 / #587 docs: 950k, not 900k).
 * Wrap+≥2hop combo overhead is dApp-only (`WRAP_ROUTER_COMBO_OVERHEAD_GAS` in
 * `totalGasLimitForExecuteMsgs`) — swarm does not compose wrap+router multi-msg.
 * Unwrap+≥2hop combo (`UNWRAP_ROUTER_COMBO_OVERHEAD_GAS`, #599) is likewise
 * dApp-only — swarm does not compose `unwrap_output` InstantWithdraw. Hop
 * floors stay in lockstep with `constants.ts`.
 */
const ROUTER_SWAP_OPS_MIN_GAS_PER_HOP = 950_000
/**
 * Keep in sync with `MIXED_HYBRID_ROUTER_HEADROOM_GAS` in
 * `frontend-dapp/src/utils/constants.ts` (GitLab #679).
 * Mixed book + pool router hops must not sum 15M per empty hop.
 */
const MIXED_HYBRID_ROUTER_HEADROOM_GAS = 2_150_000
/** Keep in sync with `SWAP_GAS_SAFETY_MARGIN` in `frontend-dapp/src/utils/constants.ts`. */
const SWAP_GAS_SAFETY_MARGIN = 10_000

function countSwapHops(msg: Record<string, unknown>): number {
  const ops = (msg as { execute_swap_operations?: { operations?: unknown[] } }).execute_swap_operations
  return ops?.operations?.length ?? 1
}

function innerSwapUsesHybrid(inner: Record<string, unknown>): boolean {
  const sw = inner.swap as { hybrid?: unknown; greedy?: unknown } | undefined
  return !!(sw && (sw.hybrid != null || sw.greedy != null))
}

function executeSwapOpsUsesHybrid(msg: Record<string, unknown>): boolean {
  const e = msg.execute_swap_operations as
    | { operations?: Array<{ terra_swap?: { hybrid?: unknown; greedy?: unknown } }> }
    | undefined
  if (!e?.operations) return false
  return e.operations.some((op) => op.terra_swap?.hybrid != null || op.terra_swap?.greedy != null)
}

function gasLimitForExecuteSwapOperations(hops: number): number {
  const hopCount = Math.max(hops, 1)
  const scaled = Math.round(600_000 * hopCount * SWAP_GAS_BUFFER)
  const padded = scaled + hopCount * SWAP_MULTIHOP_GAS_PADDING_PER_HOP
  const floor = hopCount * EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP
  return Math.max(padded, floor) + SWAP_GAS_SAFETY_MARGIN
}

function gasLimitForRouterExecuteSwapOperations(hops: number): number {
  const hopCount = Math.max(hops, 1)
  if (hopCount === 1) return ROUTER_SINGLE_HOP_GAS_LIMIT
  const scaled = Math.round(600_000 * hopCount * SWAP_GAS_BUFFER)
  const padded = scaled + hopCount * SWAP_MULTIHOP_GAS_PADDING_PER_HOP
  const floor = hopCount * ROUTER_SWAP_OPS_MIN_GAS_PER_HOP
  return Math.max(padded, floor) + SWAP_GAS_SAFETY_MARGIN
}

function bookWalkScanOverheadGas(makerUnits: number): number {
  const extraScanSteps = Math.max(0, MAX_SCAN_STEPS - makerUnits)
  return HYBRID_SWAP_PER_SCAN_STEP_GAS * extraScanSteps + HYBRID_SWAP_PER_EXPIRED_PARK_GAS * MAX_EXPIRED_PARKS_PER_SWAP
}

function gasLimitForHybridSwap(makersUsed: number): number {
  const makers = Math.max(0, Math.floor(makersUsed))
  if (makers === 0) return gasLimitForExecuteSwapOperations(1)
  const makerUnits = makers + HYBRID_SWAP_MAKER_GAS_BUFFER
  const makerComponent = HYBRID_SWAP_BASE_GAS + HYBRID_SWAP_PER_MAKER_GAS * makerUnits
  const scaled = makerComponent + bookWalkScanOverheadGas(makerUnits)
  return Math.min(HYBRID_SWAP_GAS_LIMIT, Math.max(HYBRID_SWAP_GAS_FLOOR, scaled))
}

function makersFromHybrid(hybrid: Record<string, unknown> | undefined): number | undefined {
  if (!hybrid) return undefined
  const pool = String(hybrid.pool_input ?? '0')
  const book = String(hybrid.book_input ?? '0')
  if (pool === '0' && book === '0') return undefined
  try {
    if (BigInt(book) === 0n) return 0
  } catch {
    return undefined
  }
  const fills = Number(hybrid.max_maker_fills ?? 0)
  if (!Number.isFinite(fills) || fills < 1) return undefined
  return Math.floor(fills)
}

function gasForHybridRecord(hybrid: Record<string, unknown> | undefined): number {
  const makers = makersFromHybrid(hybrid)
  if (makers === undefined) return HYBRID_SWAP_GAS_LIMIT
  return gasLimitForHybridSwap(makers)
}

function makersFromGreedy(greedy: Record<string, unknown> | undefined): number | undefined {
  if (!greedy) return undefined
  const fills = Number(greedy.max_maker_fills ?? 0)
  if (!Number.isFinite(fills) || fills < 1) return undefined
  return Math.min(100, Math.floor(fills))
}

function hopGreedyRecord(op: { terra_swap?: { greedy?: unknown } }): Record<string, unknown> | undefined {
  const greedy = op.terra_swap?.greedy
  if (greedy == null || typeof greedy !== 'object') return undefined
  return greedy as Record<string, unknown>
}

function gasForGreedyRecord(greedy: Record<string, unknown> | undefined): number {
  const makers = makersFromGreedy(greedy)
  if (makers === undefined) return HYBRID_SWAP_GAS_LIMIT
  return gasLimitForHybridSwap(makers)
}

function hopBookMakers(op: { terra_swap?: { hybrid?: unknown; greedy?: unknown } }): number | undefined {
  const greedyMakers = makersFromGreedy(hopGreedyRecord(op))
  if (greedyMakers !== undefined) return greedyMakers
  return makersFromHybrid(hopHybridRecord(op))
}

function hopHybridRecord(op: { terra_swap?: { hybrid?: unknown } }): Record<string, unknown> | undefined {
  const hybrid = op.terra_swap?.hybrid
  if (hybrid == null || typeof hybrid !== 'object') return undefined
  return hybrid as Record<string, unknown>
}

function gasLimitForSwapOperationsMsg(msg: Record<string, unknown>): number {
  const hops = countSwapHops(msg)
  const poolOnly = gasLimitForRouterExecuteSwapOperations(hops)
  if (!executeSwapOpsUsesHybrid(msg)) return poolOnly
  const e = msg.execute_swap_operations as {
    operations?: Array<{ terra_swap?: { hybrid?: unknown; greedy?: unknown } }>
  } | undefined
  const ops = e?.operations ?? []
  if (!ops.length) return HYBRID_SWAP_GAS_LIMIT

  let hasHybridField = false
  let parseableAny = 0
  let parseableBookHops = 0
  for (const op of ops) {
    if (op.terra_swap?.hybrid != null || op.terra_swap?.greedy != null) hasHybridField = true
    const makers = hopBookMakers(op)
    if (makers === undefined) continue
    parseableAny += 1
    if (makers > 0) parseableBookHops += 1
  }
  if (hasHybridField && parseableAny === 0) return HYBRID_SWAP_GAS_LIMIT

  let total = 0
  for (const op of ops) {
    const greedy = hopGreedyRecord(op)
    if (greedy) {
      total += gasForGreedyRecord(greedy)
      continue
    }
    const makers = makersFromHybrid(hopHybridRecord(op))
    if (makers !== undefined && makers > 0) {
      total += gasLimitForHybridSwap(makers)
    } else {
      total += hops <= 1 ? gasLimitForRouterExecuteSwapOperations(1) : ROUTER_SWAP_OPS_MIN_GAS_PER_HOP
    }
  }
  let limit = Math.max(poolOnly, total)
  if (parseableBookHops > 0 && parseableBookHops < hops) {
    limit += MIXED_HYBRID_ROUTER_HEADROOM_GAS
  }
  return limit
}

export function getGasLimitForExecuteMsg(executeMsg: Record<string, unknown>): number {
  if ('execute_swap_operations' in executeMsg) {
    return gasLimitForSwapOperationsMsg(executeMsg)
  }
  if ('swap' in executeMsg) {
    if (innerSwapUsesHybrid(executeMsg as Record<string, unknown>)) {
      const sw = executeMsg.swap as { hybrid?: Record<string, unknown>; greedy?: Record<string, unknown> }
      if (sw?.greedy) return gasForGreedyRecord(sw.greedy)
      return gasForHybridRecord(sw?.hybrid)
    }
    return gasLimitForExecuteSwapOperations(1)
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
        if ('place_limit_order' in inner) {
          return PLACE_LIMIT_ORDER_GAS_LIMIT
        }
        if ('place_limit_order_batch' in inner || 'place_limit_order_ladder' in inner) {
          const batch = inner.place_limit_order_batch as { orders?: unknown[] } | undefined
          const ladder = inner.place_limit_order_ladder as { ladder?: { count?: number } } | undefined
          const n = batch?.orders?.length ?? ladder?.ladder?.count ?? 1
          return PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT + PLACE_LIMIT_ORDER_BATCH_PER_RUNG_GAS_LIMIT * n
        }
        if ('swap' in inner) {
          if (innerSwapUsesHybrid(inner)) {
            const sw = inner.swap as { hybrid?: Record<string, unknown>; greedy?: Record<string, unknown> }
            if (sw?.greedy) return gasForGreedyRecord(sw.greedy)
            return gasForHybridRecord(sw?.hybrid)
          }
          return gasLimitForExecuteSwapOperations(1)
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

/** Exported for parity tests with dApp `hybridSwapGas.ts` (GitLab #260). */
export function gasLimitForHybridSwapPublic(makersUsed: number): number {
  return gasLimitForHybridSwap(makersUsed)
}
