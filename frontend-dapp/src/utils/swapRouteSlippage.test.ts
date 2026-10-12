import { describe, it, expect } from 'vitest'
import {
  SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT,
  SWAP_EXTREME_SLIPPAGE_WARNING_PCT,
  parseSlippagePercent,
  resolveRouteSlippagePercent,
  resolveSwapExpectedSlippagePercent,
  indexerWalletAmountMismatch,
  symmetricSlippagePercentFromRaw,
  preciseFairDeviationPercent,
} from './swapRouteSlippage'

describe('swapRouteSlippage (GitLab #293)', () => {
  it('prefers indexer route slippage over hop spread', () => {
    expect(resolveSwapExpectedSlippagePercent('99.50', '0.50')).toBe(99.5)
  })

  it('falls back to hop spread when route slippage missing', () => {
    expect(resolveSwapExpectedSlippagePercent(undefined, '2.10')).toBe(2.1)
  })

  it('parses slippage percent strings', () => {
    expect(parseSlippagePercent('30.01')).toBe(30.01)
    expect(parseSlippagePercent('')).toBeNull()
  })

  it('documents retail guard thresholds', () => {
    expect(SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT).toBe(30)
    expect(SWAP_EXTREME_SLIPPAGE_WARNING_PCT).toBe(99)
  })

  it('computes symmetric slippage from raw amounts', () => {
    expect(symmetricSlippagePercentFromRaw('970000', '1000000')).toBe('3.00')
  })

  it('prefers wallet receive vs indexer spot for route slippage', () => {
    expect(resolveRouteSlippagePercent('970000', '1000000', '1.00')).toBe('3.00')
    expect(resolveRouteSlippagePercent('970000', undefined, '1.00')).toBe('1.00')
  })

  it('detects indexer vs wallet amount mismatch for reconcile label (#471)', () => {
    expect(indexerWalletAmountMismatch('950000', '940000')).toBe(true)
    expect(indexerWalletAmountMismatch('940000', '940000')).toBe(false)
    expect(indexerWalletAmountMismatch('940001', '940000')).toBe(false)
  })
})

describe('exact reference protection boundaries #1483', () => {
  it.each([5, 30, 99])('retains exact %s%% threshold and both adjacent raw units', (threshold) => {
    const fair = 10n ** 36n
    const boundary = (fair * BigInt(100 - threshold)) / 100n
    expect(Number(preciseFairDeviationPercent(boundary.toString(), fair.toString()))).toBe(threshold)
    expect(Number(preciseFairDeviationPercent((boundary - 1n).toString(), fair.toString()))).toBeGreaterThan(threshold)
    expect(Number(preciseFairDeviationPercent((boundary + 1n).toString(), fair.toString()))).toBeLessThan(threshold)
    expect(Number(preciseFairDeviationPercent(fair.toString(), (boundary - 1n).toString()))).toBeGreaterThan(threshold)
  })
  it('does not accept a low server metric after malformed fair reference', () => {
    expect(resolveRouteSlippagePercent('1', 'bad', '0')).toBeUndefined()
    expect(resolveRouteSlippagePercent('1', '0', '0')).toBeUndefined()
  })
  it('does not round a 5.001% risk into 5% consent exemption', () => {
    expect(Number(resolveRouteSlippagePercent('949990', '1000000', '0'))).toBeGreaterThan(5)
  })
  it.each(['-1', 'Infinity', '5junk', '1e1', '101', 'NaN'])('rejects malformed percent %s', (raw) => {
    expect(parseSlippagePercent(raw)).toBeNull()
  })
})
