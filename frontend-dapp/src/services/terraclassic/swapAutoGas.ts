/**
 * Signed swap gas is one LCD simulate at {@link AUTO_GAS_ADJUSTMENT}, or the static
 * per-message envelope when that simulate cannot be used (#1360, ADR 0014).
 *
 * Pair addresses are not inputs. `cl8yUst1PoolOnlyTwoHopGasLimit` is not consulted.
 */
import {
  AUTO_GAS_ADJUSTMENT,
  AUTO_GAS_FALLBACK_STEP,
  AUTO_GAS_MIN_USED,
  AUTO_GAS_SIMULATE_TIMEOUT_MS,
  AUTO_GAS_STEP_TRANSPORT_LIMIT,
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
  /** Gas limit placed in the simulate tx so the query can finish. Not the signed fee. */
  queryGasLimit?: number
}

export type SimulateAttempt = 'success' | 'step' | 'stop'

/**
 * A finished simulate is a usable `gas_used` below the query budget.
 * Filling that budget means the query ran out of room, so the next attempt adds 200k.
 * Junk and over-cap figures stop the climb.
 */
export function simulateAttemptDecision(gasUsed: unknown, queryGasLimit: number): SimulateAttempt {
  if (gasUsed == null) return 'step'
  const parsed = parseAutoGasUsed(gasUsed)
  if (parsed == null) return 'stop'
  if (parsed > AUTO_GAS_SIMULATE_CAP || parsed < AUTO_GAS_MIN_USED) return 'stop'
  if (parsed >= queryGasLimit && queryGasLimit < AUTO_GAS_SIMULATE_CAP) return 'step'
  return 'success'
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
    queryGasLimit: request.queryGasLimit,
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
    const queryGas = request.queryGasLimit ?? AUTO_GAS_SIMULATE_CAP
    if (simulateAttemptDecision(value, queryGas) === 'success') {
      successCache = { key, value, at: now }
    }
    return value
  } catch {
    return null
  }
}

export type AutoGasResolution = { gasLimit: number; source: 'simulate' | 'fallback' }

/**
 * First simulate uses the 15M query ceiling. If that attempt does not finish,
 * raise the query gas from the static envelope in 200k steps until a simulate
 * returns a usable `gas_used`, then sign `ceil(gas_used × 1.2)`. Still one signature.
 * Three replies with no `gas_used` stop the climb. An included `code` 11 is not resent.
 */
export async function resolveSwapAutoGas(
  fallbackGasLimit: number,
  request: SwapGasReadRequest
): Promise<AutoGasResolution> {
  const first = await readAttempt(request, AUTO_GAS_SIMULATE_CAP)
  if (first === 'stop') return { gasLimit: fallbackGasLimit, source: 'fallback' }
  if (typeof first === 'number') return { gasLimit: first, source: 'simulate' }

  let transportFails = first === 'transport' ? 1 : 0
  for (
    let queryGas = fallbackGasLimit + AUTO_GAS_FALLBACK_STEP;
    queryGas <= AUTO_GAS_SIMULATE_CAP && transportFails < AUTO_GAS_STEP_TRANSPORT_LIMIT;
    queryGas += AUTO_GAS_FALLBACK_STEP
  ) {
    const attempt = await readAttempt(request, queryGas)
    if (attempt === 'stop') return { gasLimit: fallbackGasLimit, source: 'fallback' }
    if (typeof attempt === 'number') return { gasLimit: attempt, source: 'simulate' }
    transportFails = attempt === 'transport' ? transportFails + 1 : 0
  }
  return { gasLimit: fallbackGasLimit, source: 'fallback' }
}

async function readAttempt(
  request: SwapGasReadRequest,
  queryGasLimit: number
): Promise<number | 'transport' | 'step' | 'stop'> {
  const used = await readSwapGasUsed({ ...request, queryGasLimit })
  const decision = simulateAttemptDecision(used, queryGasLimit)
  if (decision === 'success') return autoGasWantedFromUsed(used) ?? 'stop'
  if (decision === 'stop') return 'stop'
  if (used == null) return 'transport'
  return 'step'
}

/** Swap-shaped messages simulate before the signature. Other executes keep the static envelope. */
export async function resolveBroadcastGasLimit(
  entries: AutoGasExecuteEntry[],
  signer: string
): Promise<AutoGasResolution> {
  const fallback = fallbackGasLimitForEntries(entries)
  if (!entriesUseSwapAutoGas(entries)) {
    return { gasLimit: fallback, source: 'fallback' }
  }
  return resolveSwapAutoGas(fallback, { signer, entries })
}
