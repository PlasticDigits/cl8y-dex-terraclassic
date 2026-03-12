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
 * Convert a raw on-chain integer amount to a human-readable number,
 * then format with significant figures and optional abbreviations.
 *
 *   formatTokenAmount("502498500503", 6)  → "502.5K"   (502,498.50 LUNC)
 *   formatTokenAmount("1000000", 6)       → "1"        (1 LUNC)
 */
export function formatTokenAmount(
  rawAmount: string,
  decimals: number,
  sigfigs = 4,
): string {
  if (!rawAmount || rawAmount === '0') return '0'

  let raw: bigint
  try {
    raw = BigInt(rawAmount)
  } catch {
    return '0'
  }
  const divisor = BigInt(10) ** BigInt(decimals)
  const whole = raw / divisor
  const remainder = raw % divisor
  const value = Number(whole) + Number(remainder) / Number(divisor)

  return formatNum(value, sigfigs)
}

/**
 * Resolve decimals for an AssetInfo from the token registry,
 * defaulting to 6 if the token is unknown.
 */
export function getDecimals(info: AssetInfo): number {
  return lookupByAssetInfo(info)?.decimals ?? 6
}
