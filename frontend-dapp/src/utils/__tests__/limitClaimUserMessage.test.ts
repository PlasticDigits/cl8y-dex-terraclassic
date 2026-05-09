import { describe, it, expect } from 'vitest'
import { humanizeExpiredLimitClaimMessage } from '@/utils/limitClaimUserMessage'
import { tryHumanizeTerraTxMessage } from '@/utils/humanizeTerraTxError'

describe('humanizeExpiredLimitClaimMessage', () => {
  it('maps NoExpiredLimitClaim style text to retail copy', () => {
    const raw =
      'execute wasm contract failed: No claimable expired-limit refund for order id 42: execute wasm contract failed'
    expect(humanizeExpiredLimitClaimMessage(raw)).toContain('Nothing to claim')
  })

  it('returns null for unrelated errors', () => {
    expect(humanizeExpiredLimitClaimMessage('insufficient funds')).toBeNull()
  })
})

describe('tryHumanizeTerraTxMessage + claim', () => {
  it('delegates to claim humanization', () => {
    const msg = 'No claimable expired-limit refund for order id 7'
    expect(tryHumanizeTerraTxMessage(msg)).toContain('Nothing to claim')
  })
})
