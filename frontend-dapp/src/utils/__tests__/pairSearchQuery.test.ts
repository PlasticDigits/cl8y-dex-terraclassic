import { describe, it, expect } from 'vitest'
import type { PairInfo } from '@/types'
import {
  filterFactoryPairsByLocalSearch,
  filterPairsByLocalSearch,
  isPairSearchQueryReady,
  parsePairSymbolQueryTokens,
  pairInfoSearchHaystack,
} from '@/utils/pairSearchQuery'

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

describe('parsePairSymbolQueryTokens', () => {
  it('splits space- and slash-separated pair symbols', () => {
    expect(parsePairSymbolQueryTokens('LUNC USTC')).toEqual(['lunc', 'ustc'])
    expect(parsePairSymbolQueryTokens('LUNC/USTC')).toEqual(['lunc', 'ustc'])
  })

  it('returns null for single-token queries', () => {
    expect(parsePairSymbolQueryTokens('LUNC')).toBeNull()
  })
})

const emberCoralPair: PairInfo = {
  contract_addr: 'terra1pair0000000000000000000000000000000001',
  liquidity_token: 'terra1lp',
  asset_infos: [{ token: { contract_addr: 'EMBER' } }, { token: { contract_addr: 'CORAL' } }],
}

const luncUstcPair: PairInfo = {
  contract_addr: 'terra1pair0000000000000000000000000000000002',
  liquidity_token: 'terra1lp2',
  asset_infos: [{ token: { contract_addr: 'LUNC' } }, { token: { contract_addr: 'USTC' } }],
}

describe('pairInfoSearchHaystack', () => {
  it('includes display symbols even when menu label uses shortened addresses', () => {
    const haystack = pairInfoSearchHaystack(emberCoralPair)
    expect(haystack).toContain('ember')
    expect(haystack).toContain('coral')
  })
})

describe('filterFactoryPairsByLocalSearch', () => {
  const pairs = [emberCoralPair, luncUstcPair]

  it('returns first N pairs when query is empty', () => {
    expect(filterFactoryPairsByLocalSearch(pairs, '', 1).map((p) => p.contract_addr)).toEqual([
      emberCoralPair.contract_addr,
    ])
  })

  it('filters by token symbol substring', () => {
    expect(filterFactoryPairsByLocalSearch(pairs, 'lunc', 10).map((p) => p.contract_addr)).toEqual([
      luncUstcPair.contract_addr,
    ])
  })

  it('matches two-token pair symbol queries', () => {
    expect(filterFactoryPairsByLocalSearch(pairs, 'EMBER CORAL', 10).map((p) => p.contract_addr)).toEqual([
      emberCoralPair.contract_addr,
    ])
    expect(filterFactoryPairsByLocalSearch(pairs, 'CORAL/EMBER', 10).map((p) => p.contract_addr)).toEqual([
      emberCoralPair.contract_addr,
    ])
  })

  it('matches pair contract address', () => {
    const addr = emberCoralPair.contract_addr
    expect(filterFactoryPairsByLocalSearch(pairs, addr, 10)).toHaveLength(1)
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
