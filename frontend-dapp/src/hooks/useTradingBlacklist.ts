import { useQuery } from '@tanstack/react-query'
import {
  describeTradingBlacklistBlock,
  getTradingBlacklistCheck,
  type BlacklistCheckResponse,
} from '@/services/terraclassic/blacklist'

export interface UseTradingBlacklistParams {
  wallet?: string | null
  token0?: string | null
  token1?: string | null
  pairAddress?: string | null
  pairs?: string[] | null
  enabled?: boolean
}

export function useTradingBlacklist({
  wallet,
  token0,
  token1,
  pairAddress,
  pairs,
  enabled = true,
}: UseTradingBlacklistParams) {
  const tokens = [token0, token1].filter((t): t is string => !!t && t.startsWith('terra1'))
  const pairList = pairs?.filter((p) => p.startsWith('terra1')) ?? []
  const canQuery =
    enabled &&
    !!FACTORY_OK &&
    (tokens.length > 0 || !!pairAddress || pairList.length > 0 || !!wallet?.startsWith('terra1'))

  const query = useQuery({
    queryKey: ['tradingBlacklist', wallet, tokens, pairAddress, pairList],
    queryFn: () =>
      getTradingBlacklistCheck({
        wallet: wallet?.startsWith('terra1') ? wallet : null,
        tokens,
        pair: pairAddress?.startsWith('terra1') ? pairAddress : null,
        pairs: pairList,
      }),
    enabled: canQuery,
    staleTime: 15_000,
  })

  const data = query.data
  const blocked = data?.blocked === true
  const message = blocked && data ? describeTradingBlacklistBlock(data) : null

  return {
    ...query,
    blocked,
    message,
    check: data as BlacklistCheckResponse | undefined,
  }
}

const FACTORY_OK =
  typeof import.meta.env.VITE_FACTORY_ADDRESS === 'string' &&
  import.meta.env.VITE_FACTORY_ADDRESS.trim().startsWith('terra1')
