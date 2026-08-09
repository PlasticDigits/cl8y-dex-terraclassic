import { formatFaucetCooldown } from '@/utils/faucetCooldown'

/** On-chain wrap-mapper `RateLimit` query shape (CosmWasm Timestamp may be nanos string). */
export type WrapRateLimitResponse = {
  config: { max_amount_per_window: string; window_seconds: number } | null
  current_window_start: string | { seconds: string | number; nanos?: number } | null
  amount_used: string
}

export type WrapRateLimitStatus = {
  maxRaw: bigint
  usedRaw: bigint
  /** Remaining capacity this window (full max when no window / expired). */
  remainingRaw: bigint
  windowSeconds: number
  /** Seconds until current window ends; null when no active window; 0 when expired. */
  secondsUntilReset: number | null
  windowActive: boolean
  windowExpired: boolean
}

/**
 * CosmWasm `Timestamp` serializes as a decimal nanoseconds string on LCD.
 * Also accept `{ seconds, nanos }` objects and second-scale numeric strings.
 */
export function parseCosmWasmTimestampSec(value: WrapRateLimitResponse['current_window_start']): number | null {
  if (value == null) return null
  if (typeof value === 'object') {
    const sec = Number(value.seconds)
    if (!Number.isFinite(sec) || sec < 0) return null
    return Math.floor(sec)
  }
  const raw = String(value).trim()
  if (!raw) return null
  try {
    const n = BigInt(raw)
    // ≥ 1e12 → treat as nanoseconds (CosmWasm Timestamp); else seconds.
    if (n >= 1_000_000_000_000n) {
      return Number(n / 1_000_000_000n)
    }
    const asNum = Number(n)
    if (!Number.isFinite(asNum) || asNum < 0) return null
    return Math.floor(asNum)
  } catch {
    return null
  }
}

export function deriveWrapRateLimitStatus(
  response: WrapRateLimitResponse | null | undefined,
  nowSec: number
): WrapRateLimitStatus | null {
  if (!response?.config) return null
  const maxRaw = BigInt(response.config.max_amount_per_window)
  const windowSeconds = Math.floor(Number(response.config.window_seconds))
  if (maxRaw < 0n || !Number.isFinite(windowSeconds) || windowSeconds <= 0) return null

  const startSec = parseCosmWasmTimestampSec(response.current_window_start)
  let usedRaw = 0n
  try {
    usedRaw = BigInt(response.amount_used || '0')
    if (usedRaw < 0n) usedRaw = 0n
  } catch {
    usedRaw = 0n
  }

  if (startSec == null) {
    return {
      maxRaw,
      usedRaw: 0n,
      remainingRaw: maxRaw,
      windowSeconds,
      secondsUntilReset: null,
      windowActive: false,
      windowExpired: false,
    }
  }

  const resetAt = startSec + windowSeconds
  if (nowSec >= resetAt) {
    return {
      maxRaw,
      usedRaw: 0n,
      remainingRaw: maxRaw,
      windowSeconds,
      secondsUntilReset: 0,
      windowActive: false,
      windowExpired: true,
    }
  }

  const remainingRaw = usedRaw >= maxRaw ? 0n : maxRaw - usedRaw
  return {
    maxRaw,
    usedRaw,
    remainingRaw,
    windowSeconds,
    secondsUntilReset: resetAt - nowSec,
    windowActive: true,
    windowExpired: false,
  }
}

/** Countdown label — reuse faucet formatter (`mm:ss` / `Xh Ym`). */
export function formatWrapRateLimitCountdown(secondsUntilReset: number | null): string | null {
  if (secondsUntilReset == null) return null
  if (secondsUntilReset <= 0) return 'now'
  return formatFaucetCooldown(secondsUntilReset)
}
