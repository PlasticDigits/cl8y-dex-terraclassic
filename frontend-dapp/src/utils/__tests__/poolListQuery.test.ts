import { afterEach, describe, expect, it, vi } from 'vitest'
import type { IndexerPair } from '@/types'
import {
  catalogRankAndPaginate,
  defaultOrderForPoolSort,
  filterPoolIndexerPairs,
  isPoolColumnSort,
} from '@/utils/poolListQuery'

function pair(addr: string, s0: string, s1: string, volume: string): IndexerPair {
  return {
    pair_address: addr,
    asset_0: { symbol: s0, contract_addr: `${addr}a`, denom: null, decimals: 6 },
    asset_1: { symbol: s1, contract_addr: `${addr}b`, denom: null, decimals: 6 },
    lp_token: `${addr}lp`,
    fee_bps: 30,
    is_active: true,
    volume_quote_24h: volume,
  }
}

describe('poolListQuery (GitLab #547)', () => {
  it('only allows known column sort keys (A5)', () => {
    expect(isPoolColumnSort('volume_24h')).toBe(true)
    expect(isPoolColumnSort('liquidity_usd')).toBe(true)
    expect(isPoolColumnSort('symbol')).toBe(true)
    expect(isPoolColumnSort('relevance')).toBe(false)
    expect(isPoolColumnSort('__proto__')).toBe(false)
    expect(isPoolColumnSort('tvl')).toBe(false)
  })

  it('defaults volume/fee/created/liquidity_usd to desc and name to asc', () => {
    expect(defaultOrderForPoolSort('volume_24h')).toBe('desc')
    expect(defaultOrderForPoolSort('liquidity_usd')).toBe('desc')
    expect(defaultOrderForPoolSort('fee')).toBe('desc')
    expect(defaultOrderForPoolSort('created')).toBe('desc')
    expect(defaultOrderForPoolSort('symbol')).toBe('asc')
  })

  it('catalog-ranks then paginates so UST1 leads page 1', () => {
    const items = [pair('terra1gem', 'EMBER', 'CORAL', '9'), pair('terra1ust', 'UST1', 'cUSTC', '1')]
    const { pageItems, total } = catalogRankAndPaginate(items, 0, 1)
    expect(total).toBe(2)
    expect(pageItems[0]?.asset_0.symbol).toBe('UST1')
  })
})

describe('poolListQuery production hide (GitLab #562 U6)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('catalog and volume-sort pages omit gem pairs on mainnet', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    const items = [pair('terra1gem', 'EMBER', 'CORAL', '9'), pair('terra1ust', 'UST1', 'cUSTC', '1')]
    const { pageItems, total } = catalogRankAndPaginate(items, 0, 20)
    expect(total).toBe(1)
    expect(pageItems[0]?.asset_0.symbol).toBe('UST1')
    expect(filterPoolIndexerPairs(items).every((p) => p.asset_0.symbol !== 'EMBER')).toBe(true)
  })
})
