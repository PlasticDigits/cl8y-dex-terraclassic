import { describe, expect, it } from 'vitest'
import {
  chunkExpiredClaimOrderIds,
  confirmExpiredClaimBatchMessage,
  formatExpiredClaimBatchGasLine,
  isOrderIdInExpiredClaimVariables,
  normalizeExpiredClaimOrderIds,
} from '@/utils/limitExpiredClaimBatch'

describe('limitExpiredClaimBatch (GitLab #253)', () => {
  it('normalizeExpiredClaimOrderIds dedupes, sorts, and rejects invalid ids', () => {
    expect(normalizeExpiredClaimOrderIds([3, 1, 3, 2])).toEqual([1, 2, 3])
    expect(normalizeExpiredClaimOrderIds(7)).toEqual([7])
    expect(() => normalizeExpiredClaimOrderIds([0, NaN])).toThrow(/Invalid order id/)
  })

  it('chunkExpiredClaimOrderIds splits at hard cap', () => {
    const ids = Array.from({ length: 31 }, (_, i) => i + 1)
    expect(chunkExpiredClaimOrderIds(ids)).toEqual([ids.slice(0, 30), [31]])
  })

  it('confirmExpiredClaimBatchMessage includes count, chunk progress, and gas estimate', () => {
    expect(confirmExpiredClaimBatchMessage([1, 2, 3], 0, 1, 3)).toMatch(
      /^Claim all 3 expired refund\(s\) in one transaction\? Est\. ~[\d.]+ LUNC gas \(saves ~[\d.]+ LUNC vs 3 separate claims\)\.$/
    )
    expect(confirmExpiredClaimBatchMessage([1, 2], 1, 3, 5)).toMatch(/batch 2 of 3/)
    expect(confirmExpiredClaimBatchMessage([1, 2], 1, 3, 5)).toMatch(/5 total/)
    expect(confirmExpiredClaimBatchMessage([1, 2], 1, 3, 5)).toMatch(/Est\. ~[\d.]+ LUNC gas/)
  })

  it('formatExpiredClaimBatchGasLine uses gasLimitForLimitOrderCancelBatch fee math', () => {
    for (const n of [2, 5, 30]) {
      const batchApprox = formatExpiredClaimBatchGasLine(n)
      expect(batchApprox).toMatch(/^Est\. ~[\d.]+ LUNC gas/)
      if (n >= 2) expect(batchApprox).toMatch(/saves ~[\d.]+ LUNC vs/)
    }
    const chunked = chunkExpiredClaimOrderIds(Array.from({ length: 31 }, (_, i) => i + 1))
    expect(chunked).toHaveLength(2)
    expect(confirmExpiredClaimBatchMessage(chunked[0]!, 0, 2, 31)).toMatch(/Est\. ~[\d.]+ LUNC gas/)
    expect(confirmExpiredClaimBatchMessage(chunked[1]!, 1, 2, 31)).toMatch(/batch 2 of 2/)
  })

  it('isOrderIdInExpiredClaimVariables matches scalar or batch variables', () => {
    expect(isOrderIdInExpiredClaimVariables(4, 4)).toBe(true)
    expect(isOrderIdInExpiredClaimVariables(4, [1, 4])).toBe(true)
    expect(isOrderIdInExpiredClaimVariables(4, [1, 2])).toBe(false)
    expect(isOrderIdInExpiredClaimVariables(4, undefined)).toBe(false)
  })
})
