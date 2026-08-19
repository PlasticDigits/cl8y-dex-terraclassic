import { describe, it, expect } from 'vitest'
import type { IndexerPair, IndexerTrade } from '@/types'
import {
  clampTradeDecimals,
  formatTapeAmount,
  formatTapePrice,
  resolveAskDecimals,
  resolveOfferDecimals,
  tapeRowIsBuy,
  TAPE_MISSING,
} from '../tradeTapeDisplay'

const pair: IndexerPair = {
  pair_address: 'terra1pair',
  asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
  asset_1: { symbol: 'cUSTC', contract_addr: 'terra1custc', denom: null, decimals: 6 },
  lp_token: null,
  fee_bps: 30,
  is_active: true,
}

const trade = (over: Partial<IndexerTrade> = {}): IndexerTrade => ({
  id: 1,
  pair_address: 'terra1pair',
  block_height: 1,
  block_timestamp: '2026-08-18T00:00:00Z',
  tx_hash: 'aa',
  sender: 'terra1t',
  offer_asset: 'UST1',
  ask_asset: 'cUSTC',
  offer_amount: '1000000',
  return_amount: '206000000',
  price: '206',
  ...over,
})

describe('clampTradeDecimals', () => {
  it('accepts 0–38 integers', () => {
    expect(clampTradeDecimals(0)).toBe(0)
    expect(clampTradeDecimals(18)).toBe(18)
    expect(clampTradeDecimals(38)).toBe(38)
  })

  it('rejects out of range and non-integers', () => {
    expect(clampTradeDecimals(-1)).toBeNull()
    expect(clampTradeDecimals(39)).toBeNull()
    expect(clampTradeDecimals(6.5)).toBeNull()
    expect(clampTradeDecimals('nope')).toBeNull()
  })
})

describe('formatTapeAmount', () => {
  it('humanizes UST1/cUSTC 1e6 / 2.06e8 (GitLab #557)', () => {
    expect(formatTapeAmount('1000000', 6, 'UST1')).toMatch(/^1(\.0+)? UST1$/)
    expect(formatTapeAmount('206000000', 6, 'cUSTC')).toMatch(/^206(\.0+)? cUSTC$/)
  })

  it('does not compact 18-dec USTR as T for ordinary sizes', () => {
    const out = formatTapeAmount('10000000000000000000', 18, 'USTR')
    expect(out).toMatch(/^10(\.0+)? USTR$/)
    expect(out).not.toMatch(/\dT\b/)
  })

  it('returns em dash without decimals', () => {
    expect(formatTapeAmount('1000000', null, 'UST1')).toBe(TAPE_MISSING)
  })

  it('returns em dash for non-numeric or negative', () => {
    expect(formatTapeAmount('abc', 6)).toBe(TAPE_MISSING)
    expect(formatTapeAmount('-1', 6)).toBe(TAPE_MISSING)
  })

  it('rejects scientific-notation strings (indexer must emit plain integers)', () => {
    expect(formatTapeAmount('1e+19', 18, 'USTR')).toBe(TAPE_MISSING)
  })

  it('formats zero with known decimals', () => {
    expect(formatTapeAmount('0', 6, 'UST1')).toBe('0 UST1')
  })
})

describe('formatTapePrice', () => {
  it('prints human quote-per-base without compact T', () => {
    expect(formatTapePrice('206', false)).toMatch(/^206/)
    expect(formatTapePrice('79.72', false)).toMatch(/^79\.72/)
    expect(formatTapePrice('79718100000000', false)).not.toMatch(/T$/)
  })

  it('inverts finite positive human price only', () => {
    expect(formatTapePrice('206', true)).toMatch(/^0\.00485/)
    expect(formatTapePrice('0', true)).toBe(TAPE_MISSING)
    expect(formatTapePrice('-1', true)).toBe(TAPE_MISSING)
    expect(formatTapePrice('not-a-number', false)).toBe(TAPE_MISSING)
  })
})

describe('resolveOfferDecimals', () => {
  it('prefers API decimals over pair fallback', () => {
    expect(resolveOfferDecimals(trade({ offer_decimals: 18 }), pair)).toBe(18)
  })

  it('falls back to matching pair legs only', () => {
    expect(resolveOfferDecimals(trade(), pair)).toBe(6)
    expect(resolveOfferDecimals(trade({ pair_address: 'terra1other' }), pair)).toBeNull()
  })

  it('ignores out-of-range API decimals', () => {
    expect(resolveOfferDecimals(trade({ offer_decimals: 99 }), pair)).toBe(6)
  })
})

describe('resolveAskDecimals', () => {
  it('uses ask API field', () => {
    expect(resolveAskDecimals(trade({ ask_decimals: 18 }), pair)).toBe(18)
  })
})

describe('tapeRowIsBuy', () => {
  it('treats paying factory base as a sell when not inverted', () => {
    expect(tapeRowIsBuy(trade({ offer_asset: 'UST1' }), pair, false)).toBe(false)
    expect(tapeRowIsBuy(trade({ offer_asset: 'cUSTC', ask_asset: 'UST1' }), pair, false)).toBe(true)
  })

  it('follows invert: paying UST1 while inverted is a buy of display-base', () => {
    expect(tapeRowIsBuy(trade({ offer_asset: 'UST1' }), pair, true)).toBe(true)
    expect(tapeRowIsBuy(trade({ offer_asset: 'cUSTC', ask_asset: 'UST1' }), pair, true)).toBe(false)
  })

  it('does not color mixed-pair rows', () => {
    expect(tapeRowIsBuy(trade({ pair_address: 'terra1other' }), pair, false)).toBeNull()
  })
})
