import { queryContract } from './queries'
import { FACTORY_CONTRACT_ADDRESS } from '@/utils/constants'

export interface BlacklistCheckResponse {
  blocked: boolean
  wallet_blacklisted: boolean
  blacklisted_tokens: string[]
  pair_blacklisted: boolean
  blacklisted_pairs: string[]
}

export interface BlacklistCheckParams {
  wallet?: string | null
  tokens?: string[]
  pair?: string | null
  pairs?: string[]
}

function requireFactoryAddress(): string {
  const addr = FACTORY_CONTRACT_ADDRESS.trim()
  if (!addr) {
    throw new Error('VITE_FACTORY_ADDRESS is missing')
  }
  return addr
}

/** Factory `BlacklistCheck` query (GitLab #308). */
export async function getTradingBlacklistCheck(params: BlacklistCheckParams): Promise<BlacklistCheckResponse> {
  return queryContract<BlacklistCheckResponse>(requireFactoryAddress(), {
    blacklist_check: {
      wallet: params.wallet ?? null,
      tokens: params.tokens ?? [],
      pair: params.pair ?? null,
      pairs: params.pairs ?? [],
    },
  })
}

export function describeTradingBlacklistBlock(resp: BlacklistCheckResponse): string {
  if (resp.wallet_blacklisted) {
    return (
      'This wallet is on the protocol trading blacklist for compliance or incident response. ' +
      'Swaps, liquidity, and limit orders are disabled until governance removes the restriction.'
    )
  }
  if (resp.pair_blacklisted || resp.blacklisted_pairs.length > 0) {
    return 'This pool is on the protocol trading blacklist. Trading is disabled until governance removes the restriction.'
  }
  if (resp.blacklisted_tokens.length > 0) {
    return 'A token in this pool is on the protocol trading blacklist. Trading involving that asset is disabled.'
  }
  return 'Trading is blocked by the protocol blacklist.'
}
