import { describe, expect, it } from 'vitest'
import {
  HYBRID_SWAP_GAS_LIMIT,
  gasLimitForHybridSwap,
  gasLimitForHybridParams,
  maxMakerFillsForSubmit,
} from '../hybridSwapGas'

describe('gasLimitForHybridSwap (GitLab #249)', () => {
  it('pool-only leg (0 makers) uses buffered one-hop pool envelope', () => {
    expect(gasLimitForHybridSwap({ makersUsed: 0, hasPoolLeg: true })).toBe(840_000)
  })

  it('is monotonic in makersUsed (book leg) and capped at HYBRID_SWAP_GAS_LIMIT', () => {
    const g1 = gasLimitForHybridSwap({ makersUsed: 1, hasPoolLeg: true })
    const g2 = gasLimitForHybridSwap({ makersUsed: 2, hasPoolLeg: true })
    const g10 = gasLimitForHybridSwap({ makersUsed: 10, hasPoolLeg: true })
    expect(g2).toBeLessThan(HYBRID_SWAP_GAS_LIMIT)
    expect(g1).toBeLessThan(g2)
    expect(g10).toBe(HYBRID_SWAP_GAS_LIMIT)
    expect(g10).toBeGreaterThanOrEqual(g2)
  })

  it('shallow book (2 makers) stays below flat 1.2M', () => {
    expect(gasLimitForHybridSwap({ makersUsed: 2, hasPoolLeg: true })).toBe(810_000)
  })
})

describe('gasLimitForHybridParams', () => {
  it('uses flat fallback when hybrid params are missing', () => {
    expect(gasLimitForHybridParams(undefined)).toBe(HYBRID_SWAP_GAS_LIMIT)
  })
})

describe('maxMakerFillsForSubmit', () => {
  it('adds buffer but does not exceed cap', () => {
    expect(maxMakerFillsForSubmit(8, 2)).toBe(4)
    expect(maxMakerFillsForSubmit(8, undefined)).toBe(8)
  })
})
