/**
 * Signed swap gas is one LCD simulate at {@link AUTO_GAS_ADJUSTMENT}, or the static
 * per-message envelope when that simulate cannot be used (#1360, ADR 0014).
 *
 * Pair addresses are not inputs. `cl8yUst1PoolOnlyTwoHopGasLimit` is not consulted.
 */
import {
  AUTO_GAS_ADJUSTMENT,
  AUTO_GAS_MIN_USED,
  AUTO_GAS_SIMULATE_TIMEOUT_MS,
  TERRA_LCD_URL,
  effectiveGasPriceUluna,
} from '@/utils/constants'
import { HYBRID_SWAP_GAS_LIMIT } from './hybridSwapGas'
import { estimateFeeUlunaAmountForGasLimit, getGasLimitForTx, totalGasLimitForExecuteMsgs } from './terraGas'
import { lcdReadSwapGasUsed } from './terraSimulateTx'
import type { TerraClassicFeeEstimate } from './terraClassicFeeEstimate'

export const AUTO_GAS_SIMULATE_CAP = HYBRID_SWAP_GAS_LIMIT

export type AutoGasCoin = { denom: string; amount: string }

export type AutoGasExecuteEntry = {
  contract: string
  msg: Record<string, unknown>
  coins?: AutoGasCoin[]
}

export type SwapGasReadRequest = {
  signer: string
  entries: AutoGasExecuteEntry[]
  lcdUrl?: string
  timeoutMs?: number
}

export type SwapGasReader = (request: SwapGasReadRequest) => Promise<unknown>

/** Positive integer, or null when the body is not a usable gas_used. */
export function parseAutoGasUsed(value: unknown): number | null {
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value) || value <= 0) return null
    return value
  }
  if (typeof value === 'bigint') {
    if (value <= 0n || value > BigInt(Number.MAX_SAFE_INTEGER)) return null
    return Number(value)
  }
  if (typeof value === 'string') {
    const trimmed = value.trim()
    if (!/^[0-9]+$/.test(trimmed)) return null
    const parsed = Number(trimmed)
    if (!Number.isSafeInteger(parsed) || parsed <= 0) return null
    return parsed
  }
  return null
}

/**
 * `ceil(gas_used × 1.2)` when gas_used is an integer in `[AUTO_GAS_MIN_USED, cap]`.
 * Otherwise null — the caller signs the static fallback and does not multiply.
 */
export function autoGasWantedFromUsed(gasUsed: unknown): number | null {
  const parsed = parseAutoGasUsed(gasUsed)
  if (parsed == null) return null
  if (parsed < AUTO_GAS_MIN_USED || parsed > AUTO_GAS_SIMULATE_CAP) return null
  return Math.ceil(parsed * AUTO_GAS_ADJUSTMENT)
}

export function feeEstimateForGasLimit(gasLimit: number): TerraClassicFeeEstimate {
  return {
    gasLimit,
    feeUluna: estimateFeeUlunaAmountForGasLimit(gasLimit),
    gasPriceUluna: effectiveGasPriceUluna(),
  }
}

export function fallbackGasLimitForEntries(entries: Array<{ msg: Record<string, unknown> }>): number {
  if (entries.length === 0) return 0
  if (entries.length === 1) return getGasLimitForTx(entries[0].msg)
  return totalGasLimitForExecuteMsgs(entries)
}

export function resolveAutoGasLimit(
  gasUsed: unknown,
  fallbackGasLimit: number
): { gasLimit: number; source: 'simulate' | 'fallback' } {
  const wanted = autoGasWantedFromUsed(gasUsed)
  if (wanted == null) return { gasLimit: fallbackGasLimit, source: 'fallback' }
  return { gasLimit: wanted, source: 'simulate' }
}

function sendHookInner(msg: Record<string, unknown>): Record<string, unknown> | null {
  const send = msg.send
  if (send == null || typeof send !== 'object' || Array.isArray(send)) return null
  const raw = (send as { msg?: unknown }).msg
  if (typeof raw !== 'string' || raw.length === 0) return null
  try {
    const parsed = JSON.parse(atob(raw)) as unknown
    if (parsed == null || typeof parsed !== 'object' || Array.isArray(parsed)) return null
    return parsed as Record<string, unknown>
  } catch {
    return null
  }
}

export function messageUsesSwapAutoGas(msg: Record<string, unknown>): boolean {
  if ('execute_swap_operations' in msg || 'swap' in msg || 'wrap_deposit' in msg) return true
  const inner = sendHookInner(msg)
  if (!inner) return false
  return 'execute_swap_operations' in inner || 'unwrap' in inner || 'swap' in inner
}

export function entriesUseSwapAutoGas(entries: Array<{ msg: Record<string, unknown> }>): boolean {
  return entries.some((entry) => messageUsesSwapAutoGas(entry.msg))
}

async function defaultSwapGasReader(request: SwapGasReadRequest): Promise<unknown> {
  // Unit tests must not hit a public LCD. Production and `vite dev` still simulate.
  if (import.meta.env.MODE === 'test') return null
  return lcdReadSwapGasUsed({
    lcdUrl: request.lcdUrl ?? TERRA_LCD_URL,
    signer: request.signer,
    entries: request.entries,
    timeoutMs: request.timeoutMs ?? AUTO_GAS_SIMULATE_TIMEOUT_MS,
  })
}

let swapGasReader: SwapGasReader = defaultSwapGasReader

const SUCCESS_CACHE_MS = 15_000
let successCache: { key: string; value: unknown; at: number } | null = null

function swapGasRequestKey(request: SwapGasReadRequest): string {
  return JSON.stringify({ signer: request.signer, entries: request.entries })
}

/** Test hook. Pass null to restore the LCD reader (still a no-op when MODE is test). */
export function installSwapGasReaderForTests(reader: SwapGasReader | null): void {
  swapGasReader = reader ?? defaultSwapGasReader
  successCache = null
}

export async function readSwapGasUsed(request: SwapGasReadRequest): Promise<unknown> {
  const key = swapGasRequestKey(request)
  const now = Date.now()
  if (successCache && successCache.key === key && now - successCache.at < SUCCESS_CACHE_MS) {
    return successCache.value
  }
  try {
    const value = await swapGasReader(request)
    if (autoGasWantedFromUsed(value) != null) {
      successCache = { key, value, at: now }
    }
    return value
  } catch {
    return null
  }
}

/** One simulate for swap-shaped messages. Other executes keep the static envelope. */
export async function resolveBroadcastGasLimit(
  entries: AutoGasExecuteEntry[],
  signer: string
): Promise<{ gasLimit: number; source: 'simulate' | 'fallback' }> {
  const fallback = fallbackGasLimitForEntries(entries)
  if (!entriesUseSwapAutoGas(entries)) {
    return { gasLimit: fallback, source: 'fallback' }
  }
  const used = await readSwapGasUsed({ signer, entries })
  return resolveAutoGasLimit(used, fallback)
}
