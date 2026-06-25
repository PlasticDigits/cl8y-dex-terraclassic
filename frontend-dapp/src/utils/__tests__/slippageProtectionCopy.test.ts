import { describe, expect, it } from 'vitest'
import {
  formatTransactionDeadline,
  SLIPPAGE_PROTECTION_LABEL,
  TRANSACTION_DEADLINE_LABEL,
} from '@/utils/slippageProtectionCopy'

describe('slippageProtectionCopy', () => {
  it('uses unified retail labels for user guard vs route metric', () => {
    expect(SLIPPAGE_PROTECTION_LABEL).toBe('Slippage protection')
    expect(TRANSACTION_DEADLINE_LABEL).toBe('Transaction deadline')
  })

  it('formats deadline for summary rows', () => {
    expect(formatTransactionDeadline(300)).toBe('5 min')
    expect(formatTransactionDeadline(90)).toBe('90 sec')
  })
})
