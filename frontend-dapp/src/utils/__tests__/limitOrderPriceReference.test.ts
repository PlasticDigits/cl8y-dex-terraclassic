import { describe, expect, it } from 'vitest'
import {
  anchorUsdForLimitPrice,
  escrowAmountUsdAnchorNotional,
  formatLimitPriceDeviationChipLabel,
  isLimitPriceDirectionInvalid,
  LIMIT_PRICE_DEVIATION_CHIP_PRESETS,
  LIMIT_PRICE_NEAR_MARKET_DEVIATION_PERCENT,
  limitPriceDeviationPercent,
  limitPriceFromRefDeviationChip,
  matchingLimitPriceDeviationChip,
  parsePositivePriceHuman,
  poolReservesToToken1PerToken0Human,
  resolveLimitOrderPriceRef,
  signedLimitPriceDeviationPercent,
  tradeToToken1PerToken0Human,
} from '../limitOrderPriceReference'
import type { IndexerPair, IndexerTrade, PairInfo, PoolResponse } from '@/types'

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

describe('signedLimitPriceDeviationPercent / chips (#495)', () => {
  it('maps bid chips below ref and ask chips above ref', () => {
    expect(signedLimitPriceDeviationPercent('bid', 0)).toBe(-LIMIT_PRICE_NEAR_MARKET_DEVIATION_PERCENT)
    expect(signedLimitPriceDeviationPercent('ask', 0)).toBe(LIMIT_PRICE_NEAR_MARKET_DEVIATION_PERCENT)
    expect(signedLimitPriceDeviationPercent('bid', 1)).toBe(-1)
    expect(signedLimitPriceDeviationPercent('ask', 1)).toBe(1)
    expect(signedLimitPriceDeviationPercent('bid', 10)).toBe(-10)
    expect(signedLimitPriceDeviationPercent('ask', 10)).toBe(10)
  })

  it('labels chips with maker-side signs', () => {
    expect(formatLimitPriceDeviationChipLabel('bid', 0)).toBe('0%−')
    expect(formatLimitPriceDeviationChipLabel('ask', 0)).toBe('0%+')
    expect(formatLimitPriceDeviationChipLabel('bid', 5)).toBe('−5%')
    expect(formatLimitPriceDeviationChipLabel('ask', 5)).toBe('+5%')
  })

  it('chip prices clear the #154 direction invalid gate for every preset', () => {
    const ref = 0.888
    for (const side of ['bid', 'ask'] as const) {
      for (const mag of LIMIT_PRICE_DEVIATION_CHIP_PRESETS) {
        const priceStr = limitPriceFromRefDeviationChip(side, ref, mag)
        const limit = parsePositivePriceHuman(priceStr)
        expect(limit).not.toBeNull()
        expect(isLimitPriceDirectionInvalid(side, limit!, ref)).toBe(false)
        expect(matchingLimitPriceDeviationChip(side, limit, ref)).toBe(mag)
      }
    }
  })

  it('does not match opposite-side chip magnitudes as active', () => {
    const ref = 1
    const askPlus1 = parsePositivePriceHuman(limitPriceFromRefDeviationChip('ask', ref, 1))
    expect(matchingLimitPriceDeviationChip('bid', askPlus1, ref)).toBeNull()
    const bidMinus1 = parsePositivePriceHuman(limitPriceFromRefDeviationChip('bid', ref, 1))
    expect(matchingLimitPriceDeviationChip('ask', bidMinus1, ref)).toBeNull()
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

describe('escrowAmountUsdAnchorNotional', () => {
  it('values token0 escrow as amount × headline (ask side)', () => {
    expect(escrowAmountUsdAnchorNotional(5, true, 0.888, '1.23')).toBeCloseTo(6.15, 6)
  })

  it('values token1 escrow using headline / ref (bid side)', () => {
    expect(escrowAmountUsdAnchorNotional(5, false, 0.888, '1.23')).toBeCloseTo(5 * (1.23 / 0.888), 6)
  })

  it('returns null without headline or ref', () => {
    expect(escrowAmountUsdAnchorNotional(5, false, 0.888, null)).toBeNull()
    expect(escrowAmountUsdAnchorNotional(5, true, null, '1')).toBeNull()
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

describe('poolReservesToToken1PerToken0Human', () => {
  it('matches trade-implied spot for equal-decimal reserves', () => {
    const pool = {
      assets: [
        { info: { token: { contract_addr: 'a' } }, amount: '1000000' },
        { info: { token: { contract_addr: 'b' } }, amount: '888000' },
      ],
    } as PoolResponse
    expect(poolReservesToToken1PerToken0Human(pool, 6, 6)).toBeCloseTo(0.888, 6)
  })
})

describe('resolveLimitOrderPriceRef', () => {
  const pairInfo: PairInfo = {
    contract_addr: 'pair1',
    liquidity_token: 'lp',
    asset_infos: [{ token: { contract_addr: 't0' } }, { token: { contract_addr: 't1' } }],
  }

  it('prefers tape when valid', () => {
    const tradeRow: IndexerTrade = {
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
    const r = resolveLimitOrderPriceRef({
      latestTrade: tradeRow,
      indexerPair: {
        asset_0: { symbol: 'EMBER', contract_addr: 't0', denom: null, decimals: 6 },
        asset_1: { symbol: 'CORAL', contract_addr: 't1', denom: null, decimals: 6 },
      },
      pool: null,
      pairInfo,
    })
    expect(r.refSource).toBe('tape')
    expect(r.refToken1PerToken0).toBeCloseTo(0.888, 5)
  })

  it('falls back to pool when tape missing', () => {
    const pool = {
      assets: [
        { info: { token: { contract_addr: 't0' } }, amount: '2000000' },
        { info: { token: { contract_addr: 't1' } }, amount: '1776000' },
      ],
    } as PoolResponse
    const r = resolveLimitOrderPriceRef({
      latestTrade: null,
      indexerPair: pair,
      pool,
      pairInfo,
    })
    expect(r.refSource).toBe('pool')
    expect(r.refToken1PerToken0).toBeCloseTo(0.888, 5)
  })

  it('uses decimalsOverride for pool when indexer pair row is missing', () => {
    const pool = {
      assets: [
        { info: { token: { contract_addr: 't0' } }, amount: '2000000' },
        { info: { token: { contract_addr: 't1' } }, amount: '1776000' },
      ],
    } as PoolResponse
    const r = resolveLimitOrderPriceRef({
      latestTrade: null,
      indexerPair: null,
      pool,
      pairInfo,
      decimalsOverride: { d0: 6, d1: 6 },
    })
    expect(r.refSource).toBe('pool')
    expect(r.refToken1PerToken0).toBeCloseTo(0.888, 5)
  })
})
