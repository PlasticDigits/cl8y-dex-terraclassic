import { vi } from 'vitest'
import { describeTradingBlacklistBlock, type BlacklistCheckResponse } from '@/services/terraclassic/blacklist'

/** Default `useTradingBlacklist` return when trading is allowed (GitLab #388). */
export const TRADING_BLACKLIST_ALLOWED = {
  blocked: false,
  message: null,
  isLoading: false,
  isError: false,
  isSuccess: true,
  isPending: false,
  isFetching: false,
  data: undefined,
  check: undefined,
  error: null,
  refetch: vi.fn(),
  status: 'success' as const,
}

export function tradingBlacklistHookResult(resp: BlacklistCheckResponse) {
  const blocked = resp.blocked === true
  return {
    ...TRADING_BLACKLIST_ALLOWED,
    blocked,
    message: blocked ? describeTradingBlacklistBlock(resp) : null,
    check: resp,
    data: resp,
  }
}

export function walletBlacklistedResponse(): BlacklistCheckResponse {
  return {
    blocked: true,
    wallet_blacklisted: true,
    blacklisted_tokens: [],
    pair_blacklisted: false,
    blacklisted_pairs: [],
  }
}

export function pairBlacklistedResponse(pair = 'terra1pair00000000000000000000000000000001'): BlacklistCheckResponse {
  return {
    blocked: true,
    wallet_blacklisted: false,
    blacklisted_tokens: [],
    pair_blacklisted: true,
    blacklisted_pairs: [pair],
  }
}

export function tokenBlacklistedResponse(token = 'terra1token000000000000000000000000000001'): BlacklistCheckResponse {
  return {
    blocked: true,
    wallet_blacklisted: false,
    blacklisted_tokens: [token],
    pair_blacklisted: false,
    blacklisted_pairs: [],
  }
}
