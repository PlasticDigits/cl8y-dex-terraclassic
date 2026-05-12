import { describe, expect, it } from 'vitest'
import {
  anchorUsdForLimitPrice,
  isLimitPriceDirectionInvalid,
  limitPriceDeviationPercent,
  parsePositivePriceHuman,
  tradeToToken1PerToken0Human,
} from '../limitOrderPriceReference'
import type { IndexerPair, IndexerTrade } from '@/types'

const pair: Pick<IndexerPair, 'asset_0' | 'asset_1'> = {
  asset_0: { symbol: 'EMBER', contract_addr: 't0', denom: null, decimals: 6 },
  asset_1: { symbol: 'CORAL', contract_addr: 't1', denom: null, decimals: 6 },
}

describe('tradeToToken1PerToken0Human', () => {
  it('maps token0 → token1 swap to human token1 per token0', () => {
    const trade: IndexerTrade = {
      id: 1,
      pair_address: 'p',
      block_height: 1,
      block_timestamp: '',
      tx_hash: 'h',
      sender: 's',
      offer_asset: 'EMBER',
      ask_asset: 'CORAL',
      offer_amount: '1000000',
      return_amount: '888000',
      price: '0.888',
    }
    const r = tradeToToken1PerToken0Human(trade, pair)
    expect(r).toBeCloseTo(0.888, 6)
  })

  it('maps token1 → token0 swap to human token1 per token0', () => {
    const trade: IndexerTrade = {
      id: 2,
      pair_address: 'p',
      block_height: 1,
      block_timestamp: '',
      tx_hash: 'h',
      sender: 's',
      offer_asset: 'CORAL',
      ask_asset: 'EMBER',
      offer_amount: '888000',
      return_amount: '1000000',
      price: 'x',
    }
    const r = tradeToToken1PerToken0Human(trade, pair)
    expect(r).toBeCloseTo(0.888, 6)
  })

  it('returns null when symbols do not match pair', () => {
    const trade: IndexerTrade = {
      id: 3,
      pair_address: 'p',
      block_height: 1,
      block_timestamp: '',
      tx_hash: 'h',
      sender: 's',
      offer_asset: 'X',
      ask_asset: 'Y',
      offer_amount: '1',
      return_amount: '1',
      price: '1',
    }
    expect(tradeToToken1PerToken0Human(trade, pair)).toBeNull()
  })
})

describe('isLimitPriceDirectionInvalid', () => {
  it('flags bid at or above reference', () => {
    expect(isLimitPriceDirectionInvalid('bid', 0.887, 0.888)).toBe(false)
    expect(isLimitPriceDirectionInvalid('bid', 0.888, 0.888)).toBe(true)
    expect(isLimitPriceDirectionInvalid('bid', 7, 0.888)).toBe(true)
  })

  it('flags ask at or below reference', () => {
    expect(isLimitPriceDirectionInvalid('ask', 0.889, 0.888)).toBe(false)
    expect(isLimitPriceDirectionInvalid('ask', 0.888, 0.888)).toBe(true)
    expect(isLimitPriceDirectionInvalid('ask', 0.1, 0.888)).toBe(true)
  })
})

describe('limitPriceDeviationPercent', () => {
  it('computes signed deviation', () => {
    expect(limitPriceDeviationPercent(1.776, 0.888)).toBeCloseTo(100, 3)
    expect(limitPriceDeviationPercent(0.444, 0.888)).toBeCloseTo(-50, 3)
  })
})

describe('anchorUsdForLimitPrice', () => {
  it('scales headline USD linearly with limit vs reference', () => {
    expect(anchorUsdForLimitPrice(0.888, 0.888, '1.23')).toBeCloseTo(1.23, 6)
    expect(anchorUsdForLimitPrice(1.776, 0.888, '1')).toBeCloseTo(2, 6)
  })

  it('returns null without headline', () => {
    expect(anchorUsdForLimitPrice(1, 1, null)).toBeNull()
    expect(anchorUsdForLimitPrice(1, 1, '')).toBeNull()
  })
})

describe('parsePositivePriceHuman', () => {
  it('parses positive decimals', () => {
    expect(parsePositivePriceHuman(' 0.887 ')).toBeCloseTo(0.887)
  })

  it('rejects non-positive', () => {
    expect(parsePositivePriceHuman('0')).toBeNull()
    expect(parsePositivePriceHuman('-1')).toBeNull()
    expect(parsePositivePriceHuman('abc')).toBeNull()
  })
})
