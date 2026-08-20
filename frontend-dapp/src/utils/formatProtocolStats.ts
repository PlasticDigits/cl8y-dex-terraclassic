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

/** Human FDUSD per 1 vFDUSD: missing / non-finite / ≤0 → em-dash (never Infinity, never a fake 1.0). */
export function formatProtocolFdusdOut(raw: string | number | null | undefined, digits = 6): string {
  if (raw == null || raw === '') return EM_DASH
  const n = typeof raw === 'number' ? raw : Number(raw)
  if (!Number.isFinite(n) || n <= 0) return EM_DASH
  return formatNum(raw, digits)
}
