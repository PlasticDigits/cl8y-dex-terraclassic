import type { Query } from '@tanstack/react-query'

/** True when an active query belongs to the trade workspace for the given pair address. */
export function isTradePairWorkspaceQuery(query: Query, pairAddr: string): boolean {
  const key = query.queryKey
  if (!Array.isArray(key) || key.length < 2 || key[1] !== pairAddr) return false
  const head = key[0]
  if (head === 'indexer-pair-trade' || head === 'pair-trades-trade' || head === 'candles') return true
  if (head === 'limitBookPage' && (key[2] === 'bid' || key[2] === 'ask')) return true
  return false
}
