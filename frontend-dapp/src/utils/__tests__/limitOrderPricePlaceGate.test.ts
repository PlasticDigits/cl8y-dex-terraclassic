import { describe, expect, it } from 'vitest'
import { evaluateLimitOrderPricePlaceGate } from '../limitOrderPricePlaceGate'
import type { IndexerPair, IndexerTrade } from '@/types'

const pair: Pick<IndexerPair, 'asset_0' | 'asset_1'> = {
  asset_0: { symbol: 'A', contract_addr: 'a', denom: null, decimals: 6 },
  asset_1: { symbol: 'B', contract_addr: 'b', denom: null, decimals: 6 },
}

const trade: IndexerTrade = {
  id: 1,
  pair_address: 'p',
  block_height: 1,
  block_timestamp: '',
  tx_hash: 'h',
  sender: 's',
  offer_asset: 'A',
  ask_asset: 'B',
  offer_amount: '1000000',
  return_amount: '888000',
  price: '0.888',
}

describe('evaluateLimitOrderPricePlaceGate', () => {
  it('allows placement when no trade or pair', () => {
    const r = evaluateLimitOrderPricePlaceGate('bid', '7', null, null)
    expect(r.canPlaceLimit).toBe(true)
    expect(r.userMessage).toBeNull()
  })

  it('blocks bid above reference', () => {
    const r = evaluateLimitOrderPricePlaceGate('bid', '7', trade, pair)
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toMatch(/below/i)
    expect(r.tone).toBe('error')
  })

  it('allows bid below reference', () => {
    const r = evaluateLimitOrderPricePlaceGate('bid', '0.1', trade, pair)
    expect(r.canPlaceLimit).toBe(true)
  })

  it('blocks ask below reference', () => {
    const r = evaluateLimitOrderPricePlaceGate('ask', '0.01', trade, pair)
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toMatch(/above/i)
  })
})
