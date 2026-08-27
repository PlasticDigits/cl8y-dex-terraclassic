import { formatPairPrice, formatTokenAmount, isPairLegDecimals } from './formatAmount'
import { rawLimitPriceToHuman } from './limitOrderPriceScale'
import { formatInvertedDecimal, invertFinitePositive } from './tradePairDisplayOrientation'

/** Cap USD / TWAP display strings (overview **A4** class, GitLab #564). */
const PAIR_STATS_DISPLAY_CAP = 24

/**
 * CosmWasm asset decimals are typically 0..=18. Pair JSON may send a wider i16;
 * out-of-range values must not pad/pow into a tab lock (GitLab #565).
 */
const PAIR_TOKEN_DECIMALS_MAX = 18

/**
 * Human compact token volume for Charts pair 24h stats.
 *
 * Uses **that pair leg's** decimals — never symbol matching, never assume 6.
 * Missing / non-integer / out-of-range decimals → `—`.
 * Does **not** pass raw chain integers to {@link formatNum}.
 */
export function formatChartsPairTokenVolume(
  raw: string | null | undefined,
  decimals: number | null | undefined
): string {
  if (typeof decimals !== 'number' || !Number.isInteger(decimals)) return '—'
  if (decimals < 0 || decimals > PAIR_TOKEN_DECIMALS_MAX) return '—'
  if (raw == null || raw === '') return '—'
  if (typeof raw !== 'string') return '—'
  if (raw.length > 128) return '—'
  if (!/^-?\d+$/.test(raw)) return '—'
  try {
    return formatTokenAmount(raw, decimals)
  } catch {
    return '—'
  }
}

/**
 * Convert a TWAP raw Decimal (number or decimal string) to a positive `digits.digits` string.
 * Rejects scientific notation, `<>`, and non-finite values so {@link rawLimitPriceToHuman} stays safe.
 */
export function twapRawToDecimalString(raw: string | number | null | undefined): string | null {
  if (raw == null) return null
  if (typeof raw === 'number') {
    if (!Number.isFinite(raw) || raw <= 0) return null
    if (Number.isInteger(raw) && Math.abs(raw) <= Number.MAX_SAFE_INTEGER) {
      return String(raw)
    }
    const s = raw.toLocaleString('en-US', { useGrouping: false, maximumFractionDigits: 18 })
    if (!/^\d+(\.\d+)?$/.test(s)) return null
    if (s === '0' || /^0\.0+$/.test(s)) return null
    return s
  }
  if (typeof raw !== 'string') return null
  const t = raw.trim()
  if (!t || t.length > 80 || /[<>]/.test(t)) return null
  if (!/^\d+(\.\d+)?$/.test(t)) return null
  if (t === '0' || /^0\.0+$/.test(t)) return null
  return t
}

/**
 * Pair TWAP as **human quote-per-base** (factory token1 per token0), never USD (GitLab #564 **S5**).
 *
 * On-chain `observe` Decimal is raw `reserve_b / reserve_a` (base units). Human scale is
 * `raw × 10^(decimals0 − decimals1)` via {@link rawLimitPriceToHuman}. Format with
 * {@link formatPairPrice} only (no compact `T`).
 *
 * Missing / out-of-range pair decimals, non-positive, or non-finite → `—`.
 */
export function formatTwapHumanPrice(
  raw: string | number | null | undefined,
  decimals0: unknown,
  decimals1: unknown,
  inverted = false
): string {
  if (!isPairLegDecimals(decimals0) || !isPairLegDecimals(decimals1)) return '—'
  const decimal = twapRawToDecimalString(raw)
  if (decimal == null) return '—'
  try {
    const human = rawLimitPriceToHuman(decimal, decimals0, decimals1)
    let display: string = human
    if (inverted) {
      const inv = invertFinitePositive(human)
      const formattedInv = inv == null ? null : formatInvertedDecimal(inv)
      if (formattedInv == null) return '—'
      display = formattedInv
    }
    const formatted = formatPairPrice(display, 6)
    if (!formatted || formatted === '0') return '—'
    if (/[TMBK]$/.test(formatted)) return '—'
    if (formatted.length > PAIR_STATS_DISPLAY_CAP) return '—'
    return formatted
  } catch {
    return '—'
  }
}

/**
 * Charts 24h High/Low/Open/Close **(USD)** — factory `*_usd` of 1 human asset_0 (GitLab #564 **S7**).
 * Never compact `T`/`M`/`B`. Missing / non-positive → `—`.
 */
export function formatPairStatsUsdOhlc(raw: string | null | undefined): string {
  if (raw == null || raw === '') return '—'
  const formatted = formatPairPrice(raw, 6)
  if (!formatted || formatted === '0') return '—'
  if (/[TMBK]$/.test(formatted)) return '—'
  const out = `$${formatted}`
  if (out.length > PAIR_STATS_DISPLAY_CAP) return '—'
  return out
}
