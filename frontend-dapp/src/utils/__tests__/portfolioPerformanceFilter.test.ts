import { describe, expect, it } from 'vitest'
import type { IndexerPosition, IndexerTrade } from '@/types'
import {
  countTestPositions,
  isTestPosition,
  isTestTrade,
  partitionPortfolioPositions,
  shouldOfferPortfolioTestPairsToggle,
  visiblePortfolioPositions,
  visiblePortfolioTrades,
} from '@/utils/portfolioPerformanceFilter'

const EMBER = 'terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94'
const CORAL = 'terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena'
const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'

const economic: IndexerPosition = {
  pair_address: 'terra1econ-pair',
  asset_0_symbol: 'UST1',
  asset_1_symbol: 'cUSTC',
  asset_0_decimals: 6,
  asset_1_decimals: 6,
  asset_0_denom: UST1,
  asset_1_denom: 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch',
  net_position_quote: '1000000',
  avg_entry_price: '1',
  total_cost_base: '1000000',
  realized_pnl: '25000000',
  trade_count: 2,
}

const gem: IndexerPosition = {
  pair_address: 'terra1gem-pair',
  asset_0_symbol: 'EMBER',
  asset_1_symbol: 'CORAL',
  asset_0_decimals: 6,
  asset_1_decimals: 6,
  asset_0_denom: EMBER,
  asset_1_denom: CORAL,
  net_position_quote: '9869000',
  avg_entry_price: '1.01332',
  total_cost_base: '10000000',
  realized_pnl: '20150000',
  trade_count: 3,
}

const gemBySymbolOnly: IndexerPosition = {
  pair_address: 'terra1jade-pair',
  asset_0_symbol: 'JADE',
  asset_1_symbol: 'TOPAZ',
  asset_0_decimals: 6,
  asset_1_decimals: 6,
  net_position_quote: '1',
  avg_entry_price: '1',
  total_cost_base: '1',
  realized_pnl: '1',
  trade_count: 1,
}

const cl8y: IndexerPosition = {
  pair_address: 'terra1cl8y-pair',
  asset_0_symbol: 'CL8Y',
  asset_1_symbol: 'UST1',
  asset_0_decimals: 18,
  asset_1_decimals: 6,
  net_position_quote: '1',
  avg_entry_price: '1',
  total_cost_base: '1',
  realized_pnl: '1',
  trade_count: 1,
}

function trade(offer: string, ask: string, pair = 'terra1pair'): IndexerTrade {
  return {
    id: 1,
    pair_address: pair,
    block_height: 1,
    block_timestamp: '2024-01-01T00:00:00Z',
    tx_hash: 'AABB',
    sender: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
    offer_asset: offer,
    ask_asset: ask,
    offer_amount: '1000000',
    return_amount: '1000000',
    price: '1',
  }
}

describe('portfolioPerformanceFilter (GitLab #674)', () => {
  it('classifies gem pairs by symbol or columbus-5 address (P674-1)', () => {
    expect(isTestPosition(economic)).toBe(false)
    expect(isTestPosition(cl8y)).toBe(false)
    expect(isTestPosition(gem)).toBe(true)
    expect(isTestPosition(gemBySymbolOnly)).toBe(true)
    expect(isTestPosition({ ...gem, asset_0_symbol: 'UST1', asset_1_symbol: 'UST1' })).toBe(true)
  })

  it('does not treat missing metadata as a gem (P674-6)', () => {
    expect(
      isTestPosition({
        pair_address: 'terra1orphan',
        asset_0_symbol: '—',
        asset_1_symbol: 'cUSTC',
        asset_1_decimals: 6,
        net_position_quote: '1',
        avg_entry_price: '1',
        total_cost_base: '1',
        realized_pnl: '1',
        trade_count: 1,
      })
    ).toBe(false)
  })

  it('hides gems by default and lists them after economic rows when shown (P674-2 / P674-4)', () => {
    const mixed = [gem, economic, gemBySymbolOnly, cl8y]
    expect(visiblePortfolioPositions(mixed, false).map((p) => p.pair_address)).toEqual([
      'terra1econ-pair',
      'terra1cl8y-pair',
    ])
    expect(visiblePortfolioPositions(mixed, true).map((p) => p.pair_address)).toEqual([
      'terra1econ-pair',
      'terra1cl8y-pair',
      'terra1gem-pair',
      'terra1jade-pair',
    ])
    expect(partitionPortfolioPositions(mixed).test).toHaveLength(2)
    expect(countTestPositions(mixed)).toBe(2)
  })

  it('filters recent-activity trades that touch a gem token', () => {
    const economicTrade = trade(UST1, 'uusd')
    const gemTrade = trade(EMBER, CORAL, 'terra1gem-pair')
    expect(isTestTrade(economicTrade)).toBe(false)
    expect(isTestTrade(gemTrade)).toBe(true)
    expect(visiblePortfolioTrades([gemTrade, economicTrade], false)).toEqual([economicTrade])
    expect(visiblePortfolioTrades([gemTrade, economicTrade], true)).toHaveLength(2)
  })

  it('offers the toggle only when hidden test performance exists (P674-3)', () => {
    expect(shouldOfferPortfolioTestPairsToggle([economic], [trade(UST1, 'uusd')])).toBe(false)
    expect(shouldOfferPortfolioTestPairsToggle([economic, gem], [])).toBe(true)
    expect(shouldOfferPortfolioTestPairsToggle([], [trade(EMBER, CORAL)])).toBe(true)
    expect(shouldOfferPortfolioTestPairsToggle(undefined, undefined)).toBe(false)
  })
})
