import { isValidTerraAddress } from '@/utils/constants'
import type { IndexerTrader } from '@/types'

/** Same four tabs as `/trader`. Volume must request `total_volume_usd` (T553-5 / TL-3). */
export const LEADERBOARD_TABS = [
  { key: 'total_volume_usd', label: 'Volume (USD)' },
  { key: 'best_trade_pnl', label: 'Best Trade' },
  { key: 'total_realized_pnl', label: 'Most Profit' },
  { key: 'worst_trade_pnl', label: 'Most Loss' },
] as const

/** Charts pair board hides Best Trade (CS-9). */
export const PAIR_LEADERBOARD_TABS = [
  { key: 'total_volume_usd', label: 'Volume (USD)' },
  { key: 'total_realized_pnl', label: 'Most Profit' },
  { key: 'worst_trade_pnl', label: 'Most Loss' },
] as const

export type LeaderboardSort = (typeof LEADERBOARD_TABS)[number]['key']

export const TRADER_LEADERBOARD_LIMIT = 20
export const TRADER_LEADERBOARD_REFETCH_MS = 30_000
export const DEFAULT_LEADERBOARD_SORT: LeaderboardSort = 'total_volume_usd'

const ALLOWED_SORTS = new Set<string>(LEADERBOARD_TABS.map((tab) => tab.key))

export function isLeaderboardSort(value: string): value is LeaderboardSort {
  return ALLOWED_SORTS.has(value)
}

/**
 * Drop rows that cannot be a Terra wallet link (XSS / open-redirect / prototype junk).
 * Rank stays the filtered server order — do not re-sort (TL-8).
 */
export function filterLeaderboardRows(rows: readonly unknown[] | null | undefined): IndexerTrader[] {
  if (!Array.isArray(rows)) return []
  const out: IndexerTrader[] = []
  for (const row of rows) {
    if (!row || typeof row !== 'object') continue
    const address = (row as { address?: unknown }).address
    if (typeof address !== 'string' || !isValidTerraAddress(address)) continue
    out.push(row as IndexerTrader)
  }
  return out
}

export function getLeaderboardPnlValue(trader: IndexerTrader, sort: string): string | null {
  switch (sort) {
    case 'best_trade_pnl':
      return trader.best_trade_pnl
    case 'total_realized_pnl':
      return trader.total_realized_pnl
    case 'worst_trade_pnl':
      return trader.worst_trade_pnl
    default:
      return null
  }
}
