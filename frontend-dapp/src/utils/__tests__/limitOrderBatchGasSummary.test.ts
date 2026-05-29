import { describe, expect, it } from 'vitest'

import { formatLimitBatchGasSavingsLine } from '../limitOrderBatchGasSummary'

describe('formatLimitBatchGasSavingsLine', () => {
  it('shows savings for multi-rung batch', () => {
    const line = formatLimitBatchGasSavingsLine(5, 1_300_000n, 500_000n)
    expect(line).toContain('saves')
    expect(line).toContain('5 separate')
  })

  it('omits savings for single rung', () => {
    const line = formatLimitBatchGasSavingsLine(1, 400_000n, 0n)
    expect(line).not.toContain('saves')
    expect(line).toContain('One transaction')
  })
})
