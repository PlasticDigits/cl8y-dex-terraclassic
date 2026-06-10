import { describe, expect, it } from 'vitest'
import { MIN_GAS_PRICE_ULUNA } from '@/utils/constants'
import {
  estimateTerraClassicFeeForEntries,
  estimateTerraClassicFeeForMsg,
  formatTerraClassicFeeLunc,
} from '../terraClassicFeeEstimate'

describe('terraClassicFeeEstimate (GitLab #127)', () => {
  it('estimates pool swap fee at Classic min gas price × 840k gas', () => {
    const est = estimateTerraClassicFeeForMsg({ swap: {} })
    expect(est.gasLimit).toBe(840000)
    expect(est.gasPriceUluna).toBeGreaterThanOrEqual(MIN_GAS_PRICE_ULUNA)
    expect(est.feeUluna).toBe(BigInt(Math.ceil(MIN_GAS_PRICE_ULUNA * 840000)))
  })

  it('sums multi-msg entries like broadcast path', () => {
    const est = estimateTerraClassicFeeForEntries([
      { contract: 'terra1a', msg: { wrap_deposit: {} } },
      { contract: 'terra1b', msg: { swap: {} } },
    ])
    expect(est.gasLimit).toBe(400_000 + 840_000)
    expect(est.feeUluna).toBeGreaterThan(0n)
  })

  it('formats uluna as human LUNC', () => {
    expect(formatTerraClassicFeeLunc(23793000n)).toMatch(/23/)
  })
})
