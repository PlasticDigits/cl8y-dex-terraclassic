import { describe, expect, it } from 'vitest'
import { limitOrderExpectedReceiveHuman } from '../limitOrderExpectedReceive'

describe('limitOrderExpectedReceiveHuman', () => {
  it('bid: token0 receive from token1 escrow after maker fee', () => {
    const out = limitOrderExpectedReceiveHuman({
      side: 'bid',
      escrowAmountHuman: '100',
      escrowDecimals: 6,
      priceHuman: '2',
      effectiveFeeBps: 30,
    })
    // 100 token1, 15 bps maker fee → 99.85 remaining → 49.925 token0 at price 2
    expect(out).toBe('49.9250')
  })

  it('ask: token1 receive from token0 escrow after maker fee', () => {
    const out = limitOrderExpectedReceiveHuman({
      side: 'ask',
      escrowAmountHuman: '10',
      escrowDecimals: 6,
      priceHuman: '3',
      effectiveFeeBps: 30,
    })
    // 10 token0, 15 bps fee → 9.985 remaining → 29.955 token1
    expect(out).toBe('29.9550')
  })

  it('returns null when pay amount or price is empty', () => {
    expect(
      limitOrderExpectedReceiveHuman({
        side: 'bid',
        escrowAmountHuman: '',
        escrowDecimals: 6,
        priceHuman: '2',
        effectiveFeeBps: 30,
      })
    ).toBeNull()
    expect(
      limitOrderExpectedReceiveHuman({
        side: 'ask',
        escrowAmountHuman: '1',
        escrowDecimals: 6,
        priceHuman: '',
        effectiveFeeBps: 30,
      })
    ).toBeNull()
  })
})
