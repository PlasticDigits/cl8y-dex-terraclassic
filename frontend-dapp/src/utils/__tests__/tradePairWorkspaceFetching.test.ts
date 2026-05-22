import { describe, it, expect } from 'vitest'
import type { Query } from '@tanstack/react-query'
import { isTradePairWorkspaceQuery } from '@/utils/tradePairWorkspaceFetching'

const PAIR_A = 'terra1pair0000000000000000000000000000000001'
const PAIR_B = 'terra1pair0000000000000000000000000000000002'

function mockQuery(key: readonly unknown[]): Query {
  return { queryKey: key } as Query
}

describe('isTradePairWorkspaceQuery', () => {
  it('matches indexer pair, trades, candles, and limit book for the active pair', () => {
    expect(isTradePairWorkspaceQuery(mockQuery(['indexer-pair-trade', PAIR_A]), PAIR_A)).toBe(true)
    expect(isTradePairWorkspaceQuery(mockQuery(['pair-trades-trade', PAIR_A]), PAIR_A)).toBe(true)
    expect(isTradePairWorkspaceQuery(mockQuery(['candles', PAIR_A, '1h']), PAIR_A)).toBe(true)
    expect(isTradePairWorkspaceQuery(mockQuery(['limitBookPage', PAIR_A, 'bid']), PAIR_A)).toBe(true)
    expect(isTradePairWorkspaceQuery(mockQuery(['limitBookPage', PAIR_A, 'ask']), PAIR_A)).toBe(true)
  })

  it('ignores queries for other pairs or unrelated keys', () => {
    expect(isTradePairWorkspaceQuery(mockQuery(['indexer-pair-trade', PAIR_B]), PAIR_A)).toBe(false)
    expect(isTradePairWorkspaceQuery(mockQuery(['allPairs']), PAIR_A)).toBe(false)
    expect(isTradePairWorkspaceQuery(mockQuery(['limitBookPage', PAIR_A]), PAIR_A)).toBe(false)
  })
})
