import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import type { PairInfo } from '@/types'
import {
  buildPairLocalSearchHaystack,
  buildPairSearchHaystacksByAddress,
  filterFactoryPairsByLocalSearch,
  filterPairsByLocalSearch,
  isPairSearchQueryReady,
  parsePairSymbolQueryTokens,
  pairInfoSearchHaystack,
} from '@/utils/pairSearchQuery'

const EMBER_ADDR = 'terra1ember00000000000000000000000000000001'
const CORAL_ADDR = 'terra1coral00000000000000000000000000000002'
const PAIR_ADDR = 'terra1pair0000000000000000000000000000000001'

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
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra1lp000000000000000000000000000000001',
  asset_infos: [{ token: { contract_addr: EMBER_ADDR } }, { token: { contract_addr: CORAL_ADDR } }],
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

  it('returns first N pairs when query is empty, economic catalog first (GitLab #534)', () => {
    expect(filterFactoryPairsByLocalSearch(pairs, '', 1).map((p) => p.contract_addr)).toEqual([
      luncUstcPair.contract_addr,
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
  const haystacks = new Map([
    ['terra1paira', 'AAA / BBB — terra1paira AAA BBB'],
    ['terra1pairb', 'LUNC / USTC — terra1pairb LUNC USTC TerraClassicUSD'],
  ])

  it('returns all entries when query is empty', () => {
    expect(filterPairsByLocalSearch(haystacks, '', 10)).toEqual(['terra1paira', 'terra1pairb'])
  })

  it('filters by haystack substring', () => {
    expect(filterPairsByLocalSearch(haystacks, 'lunc', 10)).toEqual(['terra1pairb'])
  })
})

describe('buildPairSearchHaystacksByAddress', () => {
  const TOKEN_CACHE_KEY = 'cl8y-dex-token-info'

  beforeEach(() => {
    localStorage.setItem(
      TOKEN_CACHE_KEY,
      JSON.stringify({
        [EMBER_ADDR.toLowerCase()]: { symbol: 'EMBER', name: 'Ember' },
        [CORAL_ADDR.toLowerCase()]: { symbol: 'CORAL', name: 'Coral' },
      })
    )
  })

  afterEach(() => {
    localStorage.removeItem(TOKEN_CACHE_KEY)
  })

  it('includes cached CW20 symbol and name for degraded typed search', () => {
    const map = buildPairSearchHaystacksByAddress([emberCoralPair])
    expect(filterPairsByLocalSearch(map, 'EMBER', 10)).toEqual([PAIR_ADDR])
    expect(filterPairsByLocalSearch(map, 'ember', 10)).toEqual([PAIR_ADDR])
    expect(filterPairsByLocalSearch(map, 'coral', 10)).toEqual([PAIR_ADDR])
    expect(filterPairsByLocalSearch(map, 'zzzzz', 10)).toEqual([])
  })

  it('matches token name in haystack when menu label uses shortened addresses (GitLab #328)', () => {
    const haystack = buildPairLocalSearchHaystack(emberCoralPair, 'terra1emb… / terra1cor… — terra1pair')
    expect(haystack.toLowerCase()).toContain('ember')
    expect(filterPairsByLocalSearch(new Map([[PAIR_ADDR, haystack]]), 'ember', 10)).toEqual([PAIR_ADDR])
  })

  it('matches cached symbols via filterFactoryPairsByLocalSearch', () => {
    expect(filterFactoryPairsByLocalSearch([emberCoralPair], 'EMBER', 10).map((p) => p.contract_addr)).toEqual([
      PAIR_ADDR,
    ])
  })
})
