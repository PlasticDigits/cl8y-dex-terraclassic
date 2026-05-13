import { describe, expect, it } from 'vitest'
import { evaluateLimitOrderPricePlaceGate } from '../limitOrderPricePlaceGate'
import type { IndexerPair, IndexerTrade } from '@/types'
import { tradeToToken1PerToken0Human } from '../limitOrderPriceReference'

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
  it('does not block when price is empty / non-positive (no typed limit yet)', () => {
    const r = evaluateLimitOrderPricePlaceGate('bid', '', null)
    expect(r.canPlaceLimit).toBe(true)
    expect(r.userMessage).toBeNull()
    const r2 = evaluateLimitOrderPricePlaceGate('bid', '0', null)
    expect(r2.canPlaceLimit).toBe(true)
  })

  it('blocks when typed limit is positive but reference is unavailable (GitLab #166)', () => {
    const r = evaluateLimitOrderPricePlaceGate('bid', '7', null)
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toMatch(/validate/i)
    expect(r.tone).toBe('error')
  })

  it('blocks while pool reference is loading', () => {
    const r = evaluateLimitOrderPricePlaceGate('bid', '1', null, { refResolutionLoading: true })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.tone).toBe('warning')
  })

  it('blocks on pool LCD error', () => {
    const r = evaluateLimitOrderPricePlaceGate('ask', '2', null, { refResolutionError: true })
    expect(r.canPlaceLimit).toBe(false)
    expect(r.tone).toBe('error')
    expect(r.userMessage).toMatch(/pool query failed/i)
  })

  it('blocks bid above reference', () => {
    const ref = tradeToToken1PerToken0Human(trade, pair)!
    const r = evaluateLimitOrderPricePlaceGate('bid', '7', ref)
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toMatch(/below/i)
    expect(r.tone).toBe('error')
  })

  it('allows bid below reference', () => {
    const ref = tradeToToken1PerToken0Human(trade, pair)!
    const r = evaluateLimitOrderPricePlaceGate('bid', '0.1', ref)
    expect(r.canPlaceLimit).toBe(true)
  })

  it('blocks ask below reference', () => {
    const ref = tradeToToken1PerToken0Human(trade, pair)!
    const r = evaluateLimitOrderPricePlaceGate('ask', '0.01', ref)
    expect(r.canPlaceLimit).toBe(false)
    expect(r.userMessage).toMatch(/above/i)
  })
})
