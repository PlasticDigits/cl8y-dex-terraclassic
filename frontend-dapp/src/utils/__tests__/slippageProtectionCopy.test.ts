import { describe, expect, it } from 'vitest'
import {
  DEFAULT_SLIPPAGE_TOLERANCE_PERCENT,
  formatTransactionDeadline,
  HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT,
  SLIPPAGE_PROTECTION_LABEL,
  SLIPPAGE_TOLERANCE_PRESETS_PERCENT,
  TRANSACTION_DEADLINE_LABEL,
} from '@/utils/slippageProtectionCopy'
import { useDexStore } from '@/stores/dex'

describe('slippageProtectionCopy', () => {
  it('uses unified retail labels for user guard vs route metric', () => {
    expect(SLIPPAGE_PROTECTION_LABEL).toBe('Slippage protection')
    expect(TRANSACTION_DEADLINE_LABEL).toBe('Transaction deadline')
  })

  it('defaults retail slippage / route-impact protection to 5% (GitLab #497)', () => {
    expect(DEFAULT_SLIPPAGE_TOLERANCE_PERCENT).toBe(5)
    expect([...SLIPPAGE_TOLERANCE_PRESETS_PERCENT]).toEqual([0.5, 1.0, 5.0])
    expect(SLIPPAGE_TOLERANCE_PRESETS_PERCENT).toContain(DEFAULT_SLIPPAGE_TOLERANCE_PERCENT)
    expect(HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT).toBe(5)
    // Reset in case another test mutated the singleton store.
    useDexStore.setState({ slippageTolerance: DEFAULT_SLIPPAGE_TOLERANCE_PERCENT })
    expect(useDexStore.getState().slippageTolerance).toBe(DEFAULT_SLIPPAGE_TOLERANCE_PERCENT)
  })

  it('formats deadline for summary rows', () => {
    expect(formatTransactionDeadline(300)).toBe('5 min')
    expect(formatTransactionDeadline(90)).toBe('90 sec')
  })
})
