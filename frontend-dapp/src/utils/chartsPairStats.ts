import { formatTokenAmount } from './formatAmount'

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
