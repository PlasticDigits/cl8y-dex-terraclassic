import { formatNum } from '@/utils/formatAmount'

const EM_DASH = '—'

/** USD headline for Protocol stats: non-finite / missing → em-dash, never Infinity. */
export function formatProtocolUsd(raw: string | number | null | undefined): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return EM_DASH
  return `$${formatNum(raw, 2)}`
}

/** Integer census figures: missing / non-finite → em-dash. */
export function formatProtocolCount(raw: number | string | null | undefined): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return EM_DASH
  return formatNum(n, 4)
}

/** Oracle snapshot: missing / non-finite → em-dash (depeg values still display). */
export function formatProtocolOracleUsd(raw: string | number | null | undefined, digits = 6): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n)) return EM_DASH
  return `$${formatNum(raw, digits)}`
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
