import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  buildTokenLocalSearchHaystack,
  filterTokensByLocalSearch,
  isTokenSearchQueryReady,
  normalizeTokenSearchQuery,
  TOKEN_SEARCH_MAX_QUERY_LENGTH,
  TOKEN_SEARCH_RESULT_LIMIT,
} from '@/utils/tokenSearchQuery'

const EMBER_ADDR = 'terra1ember00000000000000000000000000000001'
const CORAL_ADDR = 'terra1coral00000000000000000000000000000002'
const JADE_ADDR = 'terra1jade000000000000000000000000000000003'
const TOKEN_CACHE_KEY = 'cl8y-dex-token-info'

describe('isTokenSearchQueryReady', () => {
  it('allows empty query for browsing the allowed list', () => {
    expect(isTokenSearchQueryReady('')).toBe(true)
    expect(isTokenSearchQueryReady('   ')).toBe(true)
  })

  it('requires two chars unless query looks like a Terra address', () => {
    expect(isTokenSearchQueryReady('E')).toBe(false)
    expect(isTokenSearchQueryReady('EM')).toBe(true)
    expect(isTokenSearchQueryReady('terra1abc123456789012345678')).toBe(true)
  })
})

describe('normalizeTokenSearchQuery', () => {
  it('trims and truncates oversized paste', () => {
    expect(normalizeTokenSearchQuery('  ember  ')).toBe('ember')
    const long = 'x'.repeat(TOKEN_SEARCH_MAX_QUERY_LENGTH + 50)
    expect(normalizeTokenSearchQuery(long)).toHaveLength(TOKEN_SEARCH_MAX_QUERY_LENGTH)
  })
})

describe('buildTokenLocalSearchHaystack / filterTokensByLocalSearch', () => {
  beforeEach(() => {
    localStorage.setItem(
      TOKEN_CACHE_KEY,
      JSON.stringify({
        [EMBER_ADDR.toLowerCase()]: { symbol: 'EMBER', name: 'Ember Token' },
        [CORAL_ADDR.toLowerCase()]: { symbol: 'CORAL', name: 'Coral' },
        [JADE_ADDR.toLowerCase()]: { symbol: 'JADE', name: 'Jade' },
      })
    )
  })

  afterEach(() => {
    localStorage.removeItem(TOKEN_CACHE_KEY)
  })

  const tokens = [EMBER_ADDR, CORAL_ADDR, JADE_ADDR, 'uluna']

  it('includes id, display symbol, and cached name in haystack', () => {
    const haystack = buildTokenLocalSearchHaystack(EMBER_ADDR)
    expect(haystack).toContain(EMBER_ADDR.toLowerCase())
    expect(haystack).toContain('ember')
    expect(haystack).toContain('ember token')
  })

  it('empty query lists allowed tokens sorted by symbol (minus excludeToken)', () => {
    const result = filterTokensByLocalSearch(tokens, '', { excludeToken: CORAL_ADDR })
    expect(result).not.toContain(CORAL_ADDR)
    expect(result).toContain(EMBER_ADDR)
    expect(result).toContain(JADE_ADDR)
    expect(result).toContain('uluna')
    expect(result).toHaveLength(3)
    // Alphabetical by display symbol (cached): CORAL excluded → EMBER, JADE, then uluna
    expect(result.indexOf(EMBER_ADDR)).toBeLessThan(result.indexOf(JADE_ADDR))
  })

  it('filters by symbol substring case-insensitively', () => {
    expect(filterTokensByLocalSearch(tokens, 'EM')).toEqual([EMBER_ADDR])
    expect(filterTokensByLocalSearch(tokens, 'ember')).toEqual([EMBER_ADDR])
    expect(filterTokensByLocalSearch(tokens, 'jade')).toEqual([JADE_ADDR])
  })

  it('filters by cached name', () => {
    expect(filterTokensByLocalSearch(tokens, 'Ember Token')).toEqual([EMBER_ADDR])
  })

  it('matches native denom and CW20 address substrings', () => {
    expect(filterTokensByLocalSearch(tokens, 'uluna')).toEqual(['uluna'])
    expect(filterTokensByLocalSearch(tokens, EMBER_ADDR.slice(0, 20))).toEqual([EMBER_ADDR])
  })

  it('returns empty when typed query matches nothing', () => {
    expect(filterTokensByLocalSearch(tokens, 'zzzzz')).toEqual([])
  })

  it('never emits ids outside the tokens prop (selection injection guard)', () => {
    const result = filterTokensByLocalSearch([EMBER_ADDR], 'coral')
    expect(result).toEqual([])
  })

  it('excludes the other leg even when it would match the query', () => {
    expect(filterTokensByLocalSearch(tokens, 'ember', { excludeToken: EMBER_ADDR })).toEqual([])
  })

  it('caps typed results at TOKEN_SEARCH_RESULT_LIMIT', () => {
    const many = Array.from({ length: 30 }, (_, i) => {
      const addr = `terra1token${String(i).padStart(38, '0')}`
      return addr
    })
    localStorage.setItem(
      TOKEN_CACHE_KEY,
      JSON.stringify(Object.fromEntries(many.map((id) => [id.toLowerCase(), { symbol: 'AAA', name: 'Aaa' }])))
    )
    const result = filterTokensByLocalSearch(many, 'aa')
    expect(result.length).toBeLessThanOrEqual(TOKEN_SEARCH_RESULT_LIMIT)
  })
})
