import { describe, it, expect } from 'vitest'
import { describeTradingBlacklistBlock } from '../blacklist'
import {
  pairBlacklistedResponse,
  tokenBlacklistedResponse,
  walletBlacklistedResponse,
} from '@/test/tradingBlacklistMocks'

describe('describeTradingBlacklistBlock (GitLab #388 / SEC-A02)', () => {
  it('returns wallet blacklist copy', () => {
    expect(describeTradingBlacklistBlock(walletBlacklistedResponse())).toBe('Wallet blacklisted. Trading disabled.')
  })

  it('returns pair blacklist copy', () => {
    expect(describeTradingBlacklistBlock(pairBlacklistedResponse())).toBe('Pool blacklisted. Trading disabled.')
  })

  it('returns token blacklist copy', () => {
    expect(describeTradingBlacklistBlock(tokenBlacklistedResponse())).toBe('Token blacklisted. Trading disabled.')
  })

  it('returns generic fallback when blocked without a specific dimension', () => {
    expect(
      describeTradingBlacklistBlock({
        blocked: true,
        wallet_blacklisted: false,
        blacklisted_tokens: [],
        pair_blacklisted: false,
        blacklisted_pairs: [],
      })
    ).toBe('Trading blocked by blacklist.')
  })
})
