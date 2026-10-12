import { MAX_MAKER_FILLS_HARD_CAP } from './hybridBookWalkLimits'
import {
  gasLimitForRouterExecuteSwapOperations,
  gasLimitForRouteLocalSwapOperations,
  totalGasLimitForExecuteMsgs,
} from './terraGas'

/** Retail admission ceiling, shared with the existing 15M hybrid/auto-gas policy.
 * This is a client policy bound, not an attestation of chain or contract gas. */
export const OFFLINE_SWAP_GAS_CAP = 15_000_000
export const OFFLINE_SWAP_BUDGET_UNAVAILABLE = 'Network fee unavailable for this route. Refresh the quote.'
/** Current owned pair/router fixture maxima plus >=20% (issue #1480).
 * Legacy 1/2/3/4-hop maxima: 2,639,544 / 4,633,929 / 6,640,161 / 8,658,493.
 * Funding-v1 maxima: 2,121,457 / 3,596,736 / 5,083,774 / 6,578,000.
 * These measure empty-book plain CW20 settlement; hook/wrapper/device acceptance
 * is separate. Existing bounded book and wrapper overhead remains additive. */
export const OFFLINE_ROUTER_BASE_GAS = 800_000
export const OFFLINE_LEGACY_ROUTER_PER_HOP_GAS = 2_400_000
export const OFFLINE_V1_ROUTER_PER_HOP_GAS = 1_800_000
const MAX_HOOK_BASE64_LENGTH = 65_536
const UINT128_MAX = (1n << 128n) - 1n

function reject(): never {
  throw new Error(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) reject()
  return value as Record<string, unknown>
}

function uint(value: unknown, positive = false): void {
  if (
    typeof value !== 'string' ||
    !/^\d{1,39}$/.test(value) ||
    BigInt(value) > UINT128_MAX ||
    (positive && BigInt(value) === 0n)
  )
    reject()
}

function workLimit(value: unknown): void {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 1 || value > MAX_MAKER_FILLS_HARD_CAP)
    reject()
}

function keys(value: Record<string, unknown>, allowed: readonly string[]): void {
  if (Object.keys(value).some((key) => !allowed.includes(key))) reject()
}

function asset(value: unknown): void {
  const info = record(value)
  if (Object.keys(info).length !== 1) reject()
  const token = 'token' in info
  if (!token && !('native_token' in info)) reject()
  const identity = record(info[token ? 'token' : 'native_token'])
  const field = token ? 'contract_addr' : 'denom'
  keys(identity, [field])
  if (
    typeof identity[field] !== 'string' ||
    !(identity[field] as string).length ||
    (identity[field] as string).length > 128
  )
    reject()
}

function bookStartHint(value: unknown): void {
  if (value != null && (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0)) reject()
}

function validateMode(swap: Record<string, unknown>): void {
  if (swap.hybrid != null && swap.greedy != null) reject()
  if (swap.min_return != null) uint(swap.min_return, true)
  if (swap.hybrid != null) {
    const hybrid = record(swap.hybrid)
    keys(hybrid, ['pool_input', 'book_input', 'max_maker_fills', 'book_start_hint'])
    uint(hybrid.pool_input)
    uint(hybrid.book_input)
    const total = BigInt(hybrid.pool_input as string) + BigInt(hybrid.book_input as string)
    if (total === 0n || total > UINT128_MAX) reject()
    workLimit(hybrid.max_maker_fills)
    bookStartHint(hybrid.book_start_hint)
  }
  if (swap.greedy != null) {
    const greedy = record(swap.greedy)
    keys(greedy, ['max_maker_fills', 'book_start_hint'])
    workLimit(greedy.max_maker_fills)
    bookStartHint(greedy.book_start_hint)
  }
}

function validatePayload(msg: Record<string, unknown>): {
  router: boolean
  wrap: boolean
  book: boolean
  routeDelta?: number
} {
  if (Object.keys(msg).length !== 1) reject()
  if ('send' in msg) {
    const send = record(msg.send)
    keys(send, ['contract', 'amount', 'msg'])
    if (typeof send.msg !== 'string' || send.msg.length > MAX_HOOK_BASE64_LENGTH) reject()
    try {
      const inner = record(JSON.parse(atob(send.msg)))
      if ('send' in inner || 'wrap_deposit' in inner || 'wrap_deposit_with_minimum' in inner) reject()
      return validatePayload(inner)
    } catch {
      return reject()
    }
  }
  if ('execute_route_swap_operations' in msg || 'execute_swap_operations' in msg) {
    const v1 = 'execute_route_swap_operations' in msg
    const route = record(msg[v1 ? 'execute_route_swap_operations' : 'execute_swap_operations'])
    keys(route, ['funding_version', 'operations', 'max_spread', 'minimum_receive', 'to', 'deadline', 'unwrap_output'])
    if (v1 ? route.funding_version !== 1 : route.funding_version != null) reject()
    const ops = route.operations
    if (!Array.isArray(ops) || ops.length < 1 || ops.length > 4) reject()
    if (route.minimum_receive != null) uint(route.minimum_receive, true)
    if (route.unwrap_output != null && typeof route.unwrap_output !== 'boolean') reject()
    let book = false
    for (const value of ops) {
      const op = record(value)
      if (Object.keys(op).length !== 1 || !('terra_swap' in op)) reject()
      const swap = record(op.terra_swap)
      keys(swap, ['offer_asset_info', 'ask_asset_info', 'hybrid', 'greedy', 'min_return'])
      // Fee previews may use empty operation placeholders before a quote exists.
      // Exact reviewed asset/amount/floor identity remains mandatory in assertLocalRouteSubmit.
      if (swap.offer_asset_info != null) asset(swap.offer_asset_info)
      if (swap.ask_asset_info != null) asset(swap.ask_asset_info)
      validateMode(swap)
      if (swap.greedy != null && ops.length > 3) reject()
      book ||= swap.greedy != null || (swap.hybrid != null && BigInt(record(swap.hybrid).book_input as string) > 0n)
    }
    if (route.unwrap_output === true && book) reject()
    const measuredFloor =
      OFFLINE_ROUTER_BASE_GAS + ops.length * (v1 ? OFFLINE_V1_ROUTER_PER_HOP_GAS : OFFLINE_LEGACY_ROUTER_PER_HOP_GAS)
    const priorFloor = v1
      ? gasLimitForRouteLocalSwapOperations(ops.length)
      : gasLimitForRouterExecuteSwapOperations(ops.length)
    return { router: true, wrap: false, book, routeDelta: Math.max(0, measuredFloor - priorFloor) }
  }
  if ('swap' in msg) {
    const swap = record(msg.swap)
    keys(swap, [
      'offer_asset',
      'belief_price',
      'max_spread',
      'min_return',
      'minimum_receive',
      'to',
      'deadline',
      'trader',
      'hybrid',
      'greedy',
    ])
    validateMode(swap)
    return { router: false, wrap: false, book: swap.hybrid != null || swap.greedy != null }
  }
  if ('wrap_deposit' in msg || 'wrap_deposit_with_minimum' in msg) {
    const protectedWrap = 'wrap_deposit_with_minimum' in msg
    const wrap = record(msg[protectedWrap ? 'wrap_deposit_with_minimum' : 'wrap_deposit'])
    keys(wrap, protectedWrap ? ['minimum_receive'] : [])
    if (protectedWrap) uint(wrap.minimum_receive, true)
    return { router: false, wrap: true, book: false }
  }
  if ('unwrap_with_minimum' in msg) {
    const unwrap = record(msg.unwrap_with_minimum)
    keys(unwrap, ['recipient', 'minimum_receive'])
    uint(unwrap.minimum_receive, true)
    return { router: false, wrap: false, book: false }
  }
  return reject()
}

/** Validate the exact serialized local execution family before estimating or signing.
 * Unknown protocols/modes never reach the legacy BASE_GAS_LIMIT fallback. No I/O. */
export function offlineSwapGasLimit(entries: Array<{ msg: Record<string, unknown> }>): number {
  if (entries.length < 1 || entries.length > 2) reject()
  try {
    if (JSON.stringify(entries).length > MAX_HOOK_BASE64_LENGTH * 2) reject()
  } catch {
    reject()
  }
  const shapes = entries.map(({ msg }) => validatePayload(record(msg)))
  if (entries.length === 2 && (!shapes[0].wrap || !shapes[1].router || shapes[1].book)) reject()
  if (entries.length === 1 && shapes[0].wrap && 'wrap_deposit' in entries[0].msg) reject()
  const gas = totalGasLimitForExecuteMsgs(entries) + shapes.reduce((sum, shape) => sum + (shape.routeDelta ?? 0), 0)
  if (!Number.isSafeInteger(gas) || gas <= 0 || gas > OFFLINE_SWAP_GAS_CAP) reject()
  return gas
}
