import { describe, it, expect } from 'vitest'
import { filterPairsByLocalSearch, isPairSearchQueryReady } from '@/utils/pairSearchQuery'

describe('isPairSearchQueryReady', () => {
  it('allows empty query for default high-liquidity list', () => {
    expect(isPairSearchQueryReady('')).toBe(true)
    expect(isPairSearchQueryReady('   ')).toBe(true)
  })

  it('requires two chars unless query looks like a Terra address', () => {
    expect(isPairSearchQueryReady('L')).toBe(false)
    expect(isPairSearchQueryReady('LU')).toBe(true)
    expect(isPairSearchQueryReady('terra1abc123456789012345678')).toBe(true)
  })
})

describe('filterPairsByLocalSearch', () => {
  const labels = new Map([
    ['terra1paira', 'AAA / BBB — terra1paira'],
    ['terra1pairb', 'LUNC / USTC — terra1pairb'],
  ])

  it('returns all entries when query is empty', () => {
    expect(filterPairsByLocalSearch(labels, '', 10)).toEqual(['terra1paira', 'terra1pairb'])
  })

  it('filters by label substring', () => {
    expect(filterPairsByLocalSearch(labels, 'lunc', 10)).toEqual(['terra1pairb'])
  })
})
