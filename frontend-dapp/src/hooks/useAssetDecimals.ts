import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { IndexerPair } from '@/types'
import { getTokens } from '@/services/indexer/client'
import { fetchCW20TokenInfo, getCachedTokenDecimals, getCachedTokenDecimalsHostile } from '@/utils/tokenDisplay'
import { indexerTokenForId } from '@/hooks/useTokenDisplayInfo'
import {
  indexerPairLegDecimals,
  indexerTokenDecimals,
  registrySwapDecimals,
  resolveSwapAssetDecimals,
  type SwapAssetDecimalsSource,
} from '@/utils/swapAssetDecimals'

export type AssetDecimalsState = {
  decimals: number | null
  pending: boolean
  source: SwapAssetDecimalsSource | null
}

/**
 * Resolve Swap / Trade market amount decimals (Forgejo #1255).
 *
 * LCD `token_info` runs only when `lcdEnabled` — callers must pass factory/wrap
 * picker ids (`getAllTokens`), never a random pasted address (#715 **QS-4**).
 */
export function useAssetDecimals(
  tokenId: string | undefined | null,
  opts?: {
    lcdEnabled?: boolean
    indexerPair?: Pick<IndexerPair, 'asset_0' | 'asset_1'> | null
  }
): AssetDecimalsState {
  const lcdEnabled = !!opts?.lcdEnabled && !!tokenId?.toLowerCase().startsWith('terra1')
  const registryDecimals = registrySwapDecimals(tokenId)

  const { data: indexerTokens } = useQuery({
    queryKey: ['indexer-tokens-list'],
    queryFn: getTokens,
    staleTime: 5 * 60_000,
    retry: false,
  })

  const indexerRow = useMemo(
    () => (tokenId ? indexerTokenForId(tokenId, indexerTokens) : undefined),
    [tokenId, indexerTokens]
  )

  const needLcd = lcdEnabled && registryDecimals == null

  const lcdQuery = useQuery({
    queryKey: ['cw20-token-info-decimals', tokenId?.toLowerCase() ?? ''],
    queryFn: async () => {
      const info = await fetchCW20TokenInfo(tokenId!)
      if (!info) return { decimals: null as number | null, hostile: false }
      return { decimals: info.decimals, hostile: info.decimalsHostile }
    },
    enabled: needLcd,
    staleTime: 5 * 60_000,
    retry: 1,
  })

  const cachedOnChain = tokenId ? getCachedTokenDecimals(tokenId) : null
  const cachedHostile = tokenId ? getCachedTokenDecimalsHostile(tokenId) : false
  const lcdDecimals = lcdQuery.data?.decimals ?? cachedOnChain
  const lcdHostile = lcdQuery.data?.hostile === true || (lcdQuery.data == null && cachedHostile)
  const lcdSettled = !needLcd || lcdQuery.isFetched || lcdQuery.isError

  const indexerDecimals = indexerPairLegDecimals(opts?.indexerPair, tokenId) ?? indexerTokenDecimals(indexerRow)

  const resolved = resolveSwapAssetDecimals({
    registryDecimals,
    indexerDecimals,
    onChainDecimals: lcdDecimals,
    lcdSettled,
    lcdHostile,
  })

  const lcdInFlight = needLcd && !lcdQuery.isFetched && !lcdQuery.isError
  const pending = resolved.decimals == null && lcdInFlight

  return {
    decimals: resolved.decimals,
    pending,
    source: resolved.source,
  }
}
