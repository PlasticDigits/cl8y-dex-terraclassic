import { describe, it, expect } from 'vitest'
import { describeTradingBlacklistBlock } from '../blacklist'
import {
  pairBlacklistedResponse,
  tokenBlacklistedResponse,
  walletBlacklistedResponse,
} from '@/test/tradingBlacklistMocks'

describe('describeTradingBlacklistBlock (GitLab #388 / SEC-A02)', () => {
  it('returns wallet blacklist copy', () => {
    expect(describeTradingBlacklistBlock(walletBlacklistedResponse())).toBe(
      'This wallet is on the protocol trading blacklist for compliance or incident response. ' +
        'Swaps, liquidity, and limit orders are disabled until governance removes the restriction.'
    )
  })

  it('returns pair blacklist copy', () => {
    expect(describeTradingBlacklistBlock(pairBlacklistedResponse())).toBe(
      'This pool is on the protocol trading blacklist. Trading is disabled until governance removes the restriction.'
    )
  })

  it('returns token blacklist copy', () => {
    expect(describeTradingBlacklistBlock(tokenBlacklistedResponse())).toBe(
      'A token in this pool is on the protocol trading blacklist. Trading involving that asset is disabled.'
    )
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
    ).toBe('Trading is blocked by the protocol blacklist.')
  })
})
