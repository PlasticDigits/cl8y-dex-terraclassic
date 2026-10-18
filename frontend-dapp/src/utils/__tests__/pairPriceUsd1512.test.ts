import { describe, expect, it } from 'vitest'
import type { IndexerPair, IndexerTrade } from '@/types'
import {
  displayUsdPriceChangePct,
  resolveDisplayTapeLastPriceUsd,
  resolveTapeLastPriceUsd,
  resolveDisplayTapePriceUsd,
} from '../pairPriceUsd'

describe('#1512 modern indexed USD contract', () => {
  it('does not double-scale or synthesize missing USTR USD', () => {
    const input = {
      inverted: true,
      priceUsd: null,
      price: '160.756280899197382191',
      decimalsBase: 6,
      decimalsQuote: 18,
      quoteSymbol: 'USTR',
      displayBaseSymbol: 'USTR',
      ustcUsd: '0.0048565',
    }
    expect(resolveDisplayTapeLastPriceUsd(input)).toBeNull()
  })

  it.each(['0', '-1', 'NaN', 'Infinity', '1e9999', '1e-9999', '1junk', '0x10', '1<script>'])(
    'rejects malformed or nonpositive USD %s',
    (priceUsd) => {
      expect(resolveTapeLastPriceUsd({ priceUsd })).toBeNull()
    }
  )

  it('rejects arithmetic overflow/underflow and preserves a valid inversion round trip', () => {
    for (const [priceUsd, price] of [
      ['1e200', '1e-200'],
      ['1e-200', '1e200'],
    ]) {
      expect(resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd, price })).toBeNull()
    }
    const quoteUsd = resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: '6', price: '2' })
    expect(resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: quoteUsd, price: '0.5' })).toBe('6')
    expect(displayUsdPriceChangePct('1<script>', '2')).toBeNull()
  })

  it('uses human ratio once for genuine indexed factory USD', () => {
    expect(
      Number(
        resolveDisplayTapeLastPriceUsd({
          inverted: true,
          priceUsd: '0.849756570385773539',
          price: '160.756280899197382191',
        })
      )
    ).toBeCloseTo(0.005285992968, 12)
  })
})

describe('#1512 selected-pair historical provenance', () => {
  const address = 'terra1pair'
  const makePair = (d0 = 6, d1 = 18): IndexerPair => ({
    pair_address: address,
    asset_0: { symbol: 'UST1', contract_addr: 'terra1base', denom: null, decimals: d0 },
    asset_1: { symbol: 'USTR', contract_addr: 'terra1quote', denom: null, decimals: d1 },
    lp_token: 'terra1lp',
    fee_bps: 30,
    volume_quote_24h: '0',
    is_active: true,
  })
  const makeTrade = (pair: IndexerPair, reverse = false): IndexerTrade => ({
    id: 1,
    pair_address: address,
    block_height: 1,
    block_timestamp: '2026-01-01T00:00:00Z',
    tx_hash: 'AA',
    sender: 'terra1fixture',
    offer_asset: reverse ? 'USTR' : 'UST1',
    ask_asset: reverse ? 'UST1' : 'USTR',
    offer_amount: String((reverse ? 4n : 2n) * 10n ** BigInt(reverse ? pair.asset_1.decimals : pair.asset_0.decimals)),
    return_amount: String((reverse ? 2n : 4n) * 10n ** BigInt(reverse ? pair.asset_0.decimals : pair.asset_1.decimals)),
    price: '2',
    price_usd: '6',
  })

  it.each([
    [6, 6],
    [6, 18],
    [18, 6],
    [18, 18],
  ])('humanizes once with decimals %s/%s, both trade/display directions', (d0, d1) => {
    const pair = makePair(d0, d1)
    for (const reverse of [false, true])
      for (const inverted of [false, true]) {
        const trade = makeTrade(pair, reverse)
        const resolve = (row: IndexerTrade) =>
          resolveDisplayTapePriceUsd({ pair, trade: row, pairAddress: address, inverted })
        expect(resolve(trade)).toBe(inverted ? '3' : '6')
        expect(resolve({ ...trade, price: '' })).toBe(inverted ? '3' : '6')
        expect(resolve({ ...trade, price_usd: null })).toBeNull()
      }
  })

  it('rejects foreign pairs, future timestamps, malformed decimals/amounts and ratios', () => {
    const pair = makePair()
    const trade = makeTrade(pair)
    const resolve = (row = trade, market = pair) =>
      resolveDisplayTapePriceUsd({ pair: market, trade: row, pairAddress: address, inverted: true })
    expect(resolve({ ...trade, pair_address: 'terra1other' })).toBeNull()
    expect(resolve(trade, { ...pair, pair_address: 'terra1other' })).toBeNull()
    expect(resolve({ ...trade, block_timestamp: '9999-01-01T00:00:00Z' })).toBeNull()
    expect(resolve({ ...trade, block_timestamp: 'nonsense' })).toBeNull()
    expect(resolve({ ...trade, block_timestamp: '2026-02-31T00:00:00Z' })).toBeNull()
    expect(resolve({ ...trade, block_timestamp: '2026-01-01' })).toBeNull()
    for (const decimals of [-1, 1.5, 19, Number.NaN])
      expect(resolve(trade, { ...pair, asset_1: { ...pair.asset_1, decimals } })).toBeNull()
    for (const price of ['1junk', '0', 'Infinity', '1e-999', '1e999']) expect(resolve({ ...trade, price })).toBeNull()
    for (const offer_amount of ['-1', '1.1', '1e5', '9'.repeat(500)])
      expect(resolve({ ...trade, price: '', offer_amount })).toBeNull()
  })

  it('never creates USD from spoofed symbols, historical ages or an amount ratio', () => {
    const pair = makePair()
    for (const symbol of ['UST1', 'USTR', 'USDT', '<script>']) {
      expect(
        resolveDisplayTapePriceUsd({
          pair: { ...pair, asset_1: { ...pair.asset_1, symbol } },
          trade: { ...makeTrade(pair), price_usd: null },
          pairAddress: address,
          inverted: true,
        })
      ).toBeNull()
    }
  })
})
