import { describe, expect, it } from 'vitest'
import {
  HYBRID_SWAP_GAS_LIMIT,
  bookWalkScanOverheadGas,
  gasLimitForHybridSwap,
  gasLimitForHybridParams,
  maxMakerFillsForSubmit,
} from '../hybridSwapGas'
import { MAX_SCAN_STEPS, MAX_EXPIRED_PARKS_PER_SWAP } from '../hybridBookWalkLimits'

describe('gasLimitForHybridSwap (GitLab #249, #260)', () => {
  it('pool-only leg (0 makers) uses buffered one-hop pool envelope', () => {
    expect(gasLimitForHybridSwap({ makersUsed: 0, hasPoolLeg: true })).toBe(840_000)
  })

  it('is monotonic in makersUsed (book leg) and capped at HYBRID_SWAP_GAS_LIMIT', () => {
    const g1 = gasLimitForHybridSwap({ makersUsed: 1, hasPoolLeg: true })
    const g2 = gasLimitForHybridSwap({ makersUsed: 2, hasPoolLeg: true })
    const g10 = gasLimitForHybridSwap({ makersUsed: 10, hasPoolLeg: true })
    expect(g2).toBeLessThanOrEqual(HYBRID_SWAP_GAS_LIMIT)
    expect(g1).toBeLessThanOrEqual(g2)
    expect(g10).toBe(HYBRID_SWAP_GAS_LIMIT)
    expect(g10).toBeGreaterThanOrEqual(g2)
  })

  it('shallow book (2 makers) includes scan/park overhead below flat 1.2M (GitLab #260)', () => {
    const gas = gasLimitForHybridSwap({ makersUsed: 2, hasPoolLeg: true })
    expect(gas).toBeGreaterThan(810_000)
    expect(gas).toBeLessThan(HYBRID_SWAP_GAS_LIMIT)
    expect(gas).toBe(1_199_800)
  })

  it('deep maker cap (8 makers) hits HYBRID_SWAP_GAS_LIMIT before scan overhead binds', () => {
    expect(gasLimitForHybridSwap({ makersUsed: 8, hasPoolLeg: false })).toBe(HYBRID_SWAP_GAS_LIMIT)
  })
})

describe('bookWalkScanOverheadGas (GitLab #260)', () => {
  it('defaults to on-chain MAX_SCAN_STEPS and MAX_EXPIRED_PARKS_PER_SWAP', () => {
    const makerUnits = 4
    const overhead = bookWalkScanOverheadGas(makerUnits)
    expect(overhead).toBe(950 * (MAX_SCAN_STEPS - makerUnits) + 8_000 * MAX_EXPIRED_PARKS_PER_SWAP)
  })

  it('accepts optional indexer hints to tighten when head is known clean', () => {
    const overhead = bookWalkScanOverheadGas(4, { scanSteps: 6, expiredParks: 0 })
    expect(overhead).toBe(950 * 2)
  })
})

describe('gasLimitForHybridParams', () => {
  it('uses flat fallback when hybrid params are missing', () => {
    expect(gasLimitForHybridParams(undefined)).toBe(HYBRID_SWAP_GAS_LIMIT)
  })

  it('covers book leg with max_maker_fills=8 below polluted-head scan worst case (#260)', () => {
    const gas = gasLimitForHybridParams({
      pool_input: '500',
      book_input: '500',
      max_maker_fills: 8,
      book_start_hint: null,
    })
    expect(gas).toBe(HYBRID_SWAP_GAS_LIMIT)
    expect(gas).toBeGreaterThan(810_000)
  })
})

describe('maxMakerFillsForSubmit', () => {
  it('adds buffer but does not exceed cap', () => {
    expect(maxMakerFillsForSubmit(8, 2)).toBe(4)
    expect(maxMakerFillsForSubmit(8, undefined)).toBe(8)
  })
})
