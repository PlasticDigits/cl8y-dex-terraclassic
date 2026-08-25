'use strict'

const { test } = require('node:test')
const assert = require('node:assert/strict')
const {
  dailyUrl,
  mapVolume,
  mapFees,
  mapAsset,
  METHODOLOGY,
  BREAKDOWN_METHODOLOGY,
  ADAPTER_START,
} = require('./mapDaily')

test('dailyUrl pins timestamp and rejects ranges', () => {
  assert.equal(
    dailyUrl(ADAPTER_START),
    `https://indexer.dex.cl8y.com/api/v1/defillama/daily?timestamp=${ADAPTER_START}`,
  )
  assert.throws(() => dailyUrl(86401))
  assert.throws(() => dailyUrl('from=0&to=now'))
})

test('mapVolume is L10 swap_events only (null stays null)', () => {
  assert.deepEqual(mapVolume({ volume_usd: '12.5' }), { dailyVolume: 12.5 })
  assert.deepEqual(mapVolume({ volume_usd: null }), { dailyVolume: null })
  assert.deepEqual(mapVolume({ volume_usd: '0' }), { dailyVolume: 0 })
})

test('mapFees SSR is 0 and labels ⊆ breakdownMethodology', () => {
  const mapped = mapFees({
    daily_fees_usd: '2.35',
    daily_revenue_usd: '2.35',
    daily_protocol_revenue_usd: '2.35',
    fees: {
      swap_amm: '1.5',
      book_take: '0.5',
      limit_place: '0',
      wrap: '0.25',
      unwrap: '0',
      ust1_mint: '0.10',
      ust1_redeem: '0',
    },
  })
  assert.equal(mapped.dailySupplySideRevenue, 0)
  assert.equal(mapped.dailyFees, 2.35)
  assert.equal(mapped.breakdown.swap_amm, 1.5)
  for (const label of Object.keys(mapped.breakdown)) {
    assert.ok(BREAKDOWN_METHODOLOGY[label], label)
  }
  for (const key of ['Volume', 'Fees', 'Revenue', 'ProtocolRevenue', 'SupplySideRevenue']) {
    assert.ok(METHODOLOGY[key], key)
  }
})

test('mapAsset keeps UST1 unstablecoin metadata and USTR unpegged', () => {
  const json = {
    assets: {
      ust1: {
        product: 'unstablecoin',
        peg_type: 'peggedUSD',
        volume_usd: '15',
        fees_usd: '0.45',
        price_usd: '0.97',
      },
      ustr: {
        product: 'ustr',
        peg_type: null,
        volume_usd: '3',
        fees_usd: '0.08',
        price_usd: '0.015',
      },
    },
  }
  assert.deepEqual(mapAsset(json, 'ust1'), {
    ticker: 'ust1',
    volume: 15,
    fees: 0.45,
    price: 0.97,
    circulating: null,
    product: 'unstablecoin',
    pegType: 'peggedUSD',
  })
  assert.equal(mapAsset(json, 'ustr').pegType, null)
  assert.equal(mapAsset(json, 'ustr').volume, 3)
})

test('A1: mapping never reads CG liquidity_in_usd', () => {
  const fs = require('node:fs')
  const src = fs.readFileSync(require.resolve('./mapDaily'), 'utf8')
  assert.ok(!src.includes('liquidity_in_usd'))
  assert.ok(!src.includes('total_liquidity_usd'))
})
