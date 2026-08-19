import type { AssetInfo } from '@/types'
import { lookupByAssetInfo } from './tokenRegistry'

function renderSigFigs(n: number, sigfigs: number): string {
  if (n === 0) return '0'
  const sign = n < 0 ? '-' : ''
  const abs = Math.abs(n)
  const mag = Math.floor(Math.log10(abs))
  const places = Math.max(0, sigfigs - 1 - mag)
  const fixed = abs.toFixed(places)
  const [intPart, fracPart] = fixed.split('.')
  const fmtInt = Number(intPart).toLocaleString('en-US')
  return sign + (fracPart ? `${fmtInt}.${fracPart}` : fmtInt)
}

/**
 * Format a human-scale number with K / M / B / T abbreviations
 * using significant figures rather than fixed decimal places.
 *
 *   formatNum(13230, 4)       → "13.23K"
 *   formatNum(502498.5, 4)    → "502.5K"
 *   formatNum(1234567, 4)     → "1.235M"
 *   formatNum(42.195, 3)      → "42.2"
 */
export function formatNum(val: string | number, sigfigs = 4): string {
  const n = typeof val === 'string' ? parseFloat(val) : val
  if (isNaN(n) || n === 0) return '0'

  const abs = Math.abs(n)
  const tiers: [number, string][] = [
    [1e12, 'T'],
    [1e9, 'B'],
    [1e6, 'M'],
    [1e3, 'K'],
  ]

  for (const [threshold, suffix] of tiers) {
    if (abs >= threshold) {
      return renderSigFigs(n / threshold, sigfigs) + suffix
    }
  }

  return renderSigFigs(n, sigfigs)
}

/**
 * Human quote-per-base or USD spot — never compact-format as T/B/M (GitLab #522).
 * Raw mixed-decimal ratios used to render as `79.7181T`.
 */
export function formatPairPrice(val: string | number, sigfigs = 6): string {
  const n = typeof val === 'string' ? parseFloat(val) : val
  if (isNaN(n) || n === 0) return '0'
  return renderSigFigs(n, sigfigs)
}

/** Pair-leg decimals from the indexer row (GitLab #564). Never fall back to `getDecimals` (defaults to 6). */
export function isPairLegDecimals(d: unknown): d is number {
  return typeof d === 'number' && Number.isInteger(d) && d >= 0 && d <= 18
}

/** Cap compact token-volume strings so adversarial JSON cannot lock the tab (overview **A4** class). */
const PAIR_STATS_VOLUME_DISPLAY_CAP = 24

/**
 * Charts pair 24h **Vol (SYMBOL)** from indexer raw `volume_base` / `volume_quote` (GitLab #564).
 *
 * Indexer JSON stays a raw integer. This helper scales with **that pair row's** `asset_*.decimals`
 * (0…18). Compact `K`/`M`/`B`/`T` is allowed only after humanizing.
 *
 * - Zero raw → `0`
 * - Missing / non-integer / out-of-range decimals, non-numeric, negative, `<>` → `—`
 */
export function formatPairStatsVolume(raw: string | null | undefined, decimals: unknown): string {
  if (!isPairLegDecimals(decimals)) return '—'
  if (raw == null || raw === '') return '—'
  if (typeof raw !== 'string') return '—'
  if (raw.length > 78) return '—'
  if (/[<>]/.test(raw)) return '—'
  let n: bigint
  try {
    n = BigInt(raw)
  } catch {
    return '—'
  }
  if (n < 0n) return '—'
  if (n === 0n) return '0'
  const formatted = formatTokenAmount(raw, decimals)
  if (!formatted || formatted.length > PAIR_STATS_VOLUME_DISPLAY_CAP) return '—'
  return formatted
}

/**
 * Format indexer `volume_quote_24h` (raw quote-side integer) using the quote token's decimals.
 * Passing the raw 18-dec string to {@link formatNum} prints `19,297,048T` for ordinary USTR flow (GitLab #534).
 *
 * Returns null when the raw amount is missing or zero so callers can hide the badge.
 */
export function formatQuoteVolume24h(
  raw: string | undefined | null,
  quoteDecimals: number,
  sigfigs = 3
): string | null {
  if (!raw || raw === '0') return null
  let n: bigint
  try {
    n = BigInt(raw)
  } catch {
    return null
  }
  if (n <= 0n) return null
  const formatted = formatTokenAmount(raw, quoteDecimals, sigfigs)
  return formatted === '0' ? null : formatted
}

/**
 * Convert a raw on-chain integer amount to a human-readable number,
 * then format with significant figures and optional abbreviations.
 *
 *   formatTokenAmount("502498500503", 6)  → "502.5K"   (502,498.50 LUNC)
 *   formatTokenAmount("1000000", 6)       → "1"        (1 LUNC)
 */
export function formatTokenAmount(rawAmount: string, decimals: number, sigfigs = 4): string {
  if (!rawAmount || rawAmount === '0') return '0'

  let raw: bigint
  try {
    raw = BigInt(rawAmount)
  } catch {
    return '0'
  }

  const humanStr = fromRawAmount(rawAmount, decimals)
  const absHuman = humanStr.startsWith('-') ? humanStr.slice(1) : humanStr
  const intPart = absHuman.split('.')[0] || '0'
  if (BigInt(intPart || '0') <= BigInt(Number.MAX_SAFE_INTEGER)) {
    return formatNum(humanStr, sigfigs)
  }

  const divisor = BigInt(10) ** BigInt(decimals)
  const whole = raw / divisor
  const remainder = raw % divisor
  const sign = whole < 0n ? '-' : ''
  const absWhole = whole < 0n ? -whole : whole

  if (remainder === 0n) {
    return formatWholeTokensAbbrev(absWhole, sign)
  }

  return formatNum(humanStr, sigfigs)
}

/**
 * Convert a human-readable token amount string to raw on-chain micro-units.
 * Handles decimal inputs precisely using string manipulation to avoid
 * floating-point rounding.
 *
 *   toRawAmount("10000", 6)   → "10000000000"
 *   toRawAmount("1.5", 6)     → "1500000"
 *   toRawAmount("0.001", 6)   → "1000"
 */
export function toRawAmount(humanAmount: string, decimals: number): string {
  if (!humanAmount || humanAmount === '0') return '0'
  const [intPart, fracPart = ''] = humanAmount.split('.')
  const paddedFrac = fracPart.padEnd(decimals, '0').slice(0, decimals)
  const result = (intPart + paddedFrac).replace(/^0+/, '') || '0'
  return result
}

/**
 * Convert a raw on-chain micro-unit amount to a plain decimal string
 * (no abbreviations, no formatting — suitable for input fields).
 *
 *   fromRawAmount("10000000000", 6)  → "10000"
 *   fromRawAmount("1500000", 6)      → "1.5"
 *   fromRawAmount("1000", 6)         → "0.001"
 */
export function fromRawAmount(rawAmount: string, decimals: number): string {
  if (!rawAmount || rawAmount === '0') return '0'
  const padded = rawAmount.padStart(decimals + 1, '0')
  const intPart = padded.slice(0, padded.length - decimals) || '0'
  const fracPart = padded.slice(padded.length - decimals)
  const trimmedFrac = fracPart.replace(/0+$/, '')
  return trimmedFrac ? `${intPart}.${trimmedFrac}` : intPart
}

function stripTrailingZerosAbbrevDisplay(s: string): string {
  const suffix = /[KMBT]$/.exec(s)?.[0] ?? ''
  const core = suffix ? s.slice(0, -1) : s
  if (/^\d+(\.\d+)?$/.test(core)) {
    const [intPart, fracPart = ''] = core.split('.')
    const trimmedFrac = fracPart.replace(/0+$/, '')
    const fmtInt = Number(intPart).toLocaleString('en-US')
    if (!trimmedFrac) {
      return fmtInt + suffix
    }
    return `${fmtInt}.${trimmedFrac}` + suffix
  }
  const n = parseFloat(core.replace(/,/g, ''))
  if (isNaN(n)) return s
  if (n === 0 && !suffix) return '0'
  if (Number.isInteger(n)) {
    return n.toLocaleString('en-US', { maximumFractionDigits: 0, useGrouping: true }) + suffix
  }
  const formatted = n.toLocaleString('en-US', { maximumFractionDigits: 12, useGrouping: true })
  return formatted + suffix
}

function bigintAbbrevCoef(absWhole: bigint, threshold: bigint): string {
  const scaledInt = absWhole / threshold
  const rem = absWhole % threshold
  if (rem === 0n) return scaledInt.toString()
  const fracDigits = threshold.toString().length - 1
  const frac = rem.toString().padStart(fracDigits, '0').replace(/0+$/, '')
  return frac ? `${scaledInt}.${frac}` : scaledInt.toString()
}

function formatWholeTokensAbbrev(absWhole: bigint, sign: string): string {
  if (absWhole === 0n) return '0'
  const tiers: [bigint, string][] = [
    [10n ** 12n, 'T'],
    [10n ** 9n, 'B'],
    [10n ** 6n, 'M'],
    [10n ** 3n, 'K'],
  ]
  for (const [th, suf] of tiers) {
    if (absWhole >= th) {
      const scaledInt = absWhole / th
      const rem = absWhole % th
      if (rem === 0n) {
        return `${sign}${scaledInt.toLocaleString('en-US')}${suf}`
      }
      const coef = stripTrailingZerosAbbrevDisplay(bigintAbbrevCoef(absWhole, th))
      return `${sign}${coef}${suf}`
    }
  }
  return `${sign}${absWhole.toLocaleString('en-US')}`
}

/**
 * Like formatTokenAmount (K/M/B/T) but omits meaningless decimals for whole
 * token counts (e.g. "1" and "5K" instead of "1.000" and "5.000K").
 */
export function formatTokenAmountAbbrev(rawAmount: string, decimals: number, sigfigs = 4): string {
  if (!rawAmount || rawAmount === '0') return '0'
  let raw: bigint
  try {
    raw = BigInt(rawAmount)
  } catch {
    return '0'
  }
  if (raw === 0n) return '0'

  const divisor = 10n ** BigInt(decimals)
  const whole = raw / divisor
  const remainder = raw % divisor

  const sign = whole < 0n ? '-' : ''
  const absWhole = whole < 0n ? -whole : whole

  if (remainder === 0n) {
    return formatWholeTokensAbbrev(absWhole, sign)
  }

  const humanStr = fromRawAmount(rawAmount, decimals)
  const absHuman = humanStr.startsWith('-') ? humanStr.slice(1) : humanStr
  const intPart = absHuman.split('.')[0] || '0'
  if (BigInt(intPart || '0') <= BigInt(Number.MAX_SAFE_INTEGER)) {
    return stripTrailingZerosAbbrevDisplay(formatNum(humanStr, sigfigs))
  }

  return `${sign}${absWhole.toLocaleString('en-US')}`
}

/**
 * Resolve decimals for an AssetInfo from the token registry,
 * defaulting to 6 if the token is unknown.
 */
export function getDecimals(info: AssetInfo): number {
  return lookupByAssetInfo(info)?.decimals ?? 6
}
