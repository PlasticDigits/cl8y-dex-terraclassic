import { describe, it, expect } from 'vitest'
import {
  SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT,
  SWAP_EXTREME_SLIPPAGE_WARNING_PCT,
  parseSlippagePercent,
  resolveSwapExpectedSlippagePercent,
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
})
