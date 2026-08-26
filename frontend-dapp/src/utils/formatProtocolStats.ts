import { formatNum } from '@/utils/formatAmount'

const EM_DASH = '—'

/** USD headline for Protocol stats: non-finite / missing → em-dash, never Infinity. */
export function formatProtocolUsd(raw: string | number | null | undefined): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return EM_DASH
  return `$${formatNum(raw, 2)}`
}

/**
 * Integer census figures (tokens / pairs / trades). Missing / non-finite /
 * negative / XSS-like → em-dash. Finite integers under 1000 are locale counts
 * (`14`, not `14.00`). Compact `K`/`M` only when `abs >= 1e3` (GitLab #667).
 */
export function formatProtocolCount(raw: number | string | null | undefined): string {
  if (raw == null || raw === '') return EM_DASH
  if (typeof raw === 'string' && /[<>]|javascript:/i.test(raw)) return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n) || n < 0) return EM_DASH
  const whole = Math.round(n)
  if (!Number.isFinite(whole) || whole < 0) return EM_DASH
  if (Math.abs(whole) >= 1e3) return formatNum(whole, 4)
  return whole.toLocaleString('en-US')
}

/** Oracle snapshot: missing / non-finite → em-dash (depeg values still display). */
export function formatProtocolOracleUsd(raw: string | number | null | undefined, digits = 6): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return EM_DASH
  return `$${formatNum(raw, digits)}`
}

/** Human FDUSD per 1 vFDUSD: missing / non-finite / ≤0 → em-dash (never Infinity, never a fake 1.0). */
export function formatProtocolFdusdOut(raw: string | number | null | undefined, digits = 6): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n) || n <= 0) return EM_DASH
  return formatNum(raw, digits)
}

/** Signed compact percent for Protocol liquidity Δ%. Missing / non-finite → em-dash; never Infinity. */
export function formatProtocolPct(raw: string | number | null | undefined): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return EM_DASH
  if (n === 0) return '0%'
  const compact = formatNum(n, 4)
  return n > 0 ? `+${compact}%` : `${compact}%`
}

/** Semantic tone for an already-formatted Δ% (`formatProtocolPct`). Gold is never a fill. */
export function protocolPctToneFromDisplay(display: string): string {
  if (display === EM_DASH || display === '0%') return 'var(--ink-dim)'
  if (display.startsWith('+')) return 'var(--color-positive)'
  if (display.startsWith('-')) return 'var(--color-negative)'
  return 'var(--ink-dim)'
}
