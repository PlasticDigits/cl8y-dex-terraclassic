import { describe, expect, it } from 'vitest'
import {
  DEFAULT_SLIPPAGE_TOLERANCE_PERCENT,
  formatTransactionDeadline,
  HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT,
  isSlippageCustomOutOfRange,
  maxSpreadFromSlippagePercent,
  persistSlippageCustomInput,
  sanitizeSlippageCustomInput,
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

  it('maps retail percent to on-chain max_spread strings (GitLab #528 AC5)', () => {
    expect(maxSpreadFromSlippagePercent(5)).toBe('0.05')
    expect(maxSpreadFromSlippagePercent(0.5)).toBe('0.005')
    expect(maxSpreadFromSlippagePercent(1)).toBe('0.01')
  })

  it('sanitizes Custom input so injection / scientific notation cannot persist raw (GitLab #528 A4)', () => {
    expect(sanitizeSlippageCustomInput('1e9')).toBe('19')
    expect(sanitizeSlippageCustomInput('0x1')).toBe('01')
    expect(sanitizeSlippageCustomInput('1; DROP')).toBe('1')
    expect(sanitizeSlippageCustomInput('５')).toBe('')
    expect(sanitizeSlippageCustomInput('5.5.5')).toBe('5.55')
    expect(sanitizeSlippageCustomInput('12.3abc')).toBe('12.3')
    expect(persistSlippageCustomInput(sanitizeSlippageCustomInput('1e9'))).toBe(19)
    expect(persistSlippageCustomInput('1e9')).toBe(50)
    expect(persistSlippageCustomInput('0')).toBeNull()
    expect(persistSlippageCustomInput('99')).toBe(50)
    expect(persistSlippageCustomInput('2.5')).toBe(2.5)
    expect(isSlippageCustomOutOfRange('0')).toBe(true)
    expect(isSlippageCustomOutOfRange('99')).toBe(true)
    expect(isSlippageCustomOutOfRange('5')).toBe(false)
    expect(isSlippageCustomOutOfRange('')).toBe(false)
  })
})
