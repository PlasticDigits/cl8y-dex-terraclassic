import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { AssetInfo, IndexerToken } from '@/types'
import { getTokens } from '@/services/indexer/client'
import {
  getCachedTokenSymbol,
  fetchCW20TokenInfo,
  getTokenLogoURI,
  shortenAddress,
  isAddressLike,
} from '@/utils/tokenDisplay'

export interface TokenDisplayInfo {
  displayLabel: string
  symbol: string
  addressForBlockie: string | undefined
  logoURI: string | undefined
}

function indexerTokenForId(tokenId: string, list: IndexerToken[] | undefined) {
  if (!list?.length || !tokenId) return undefined
  const t = tokenId.toLowerCase()
  return list.find(
    (x) => (x.contract_address && x.contract_address.toLowerCase() === t) || (x.denom && x.denom === tokenId)
  )
}

export function useTokenDisplayInfo(info: AssetInfo | null): TokenDisplayInfo {
  const tokenId = info ? ('token' in info ? info.token.contract_addr : info.native_token.denom) : ''
  const isCw20 = !!info && 'token' in info

  const { data: indexerTokens } = useQuery({
    queryKey: ['indexer-tokens-list'],
    queryFn: getTokens,
    staleTime: 5 * 60_000,
    retry: false,
  })

  const indexerMeta = useMemo(() => indexerTokenForId(tokenId, indexerTokens), [tokenId, indexerTokens])

  const [resolved, setResolved] = useState<string | null>(() => (tokenId ? getCachedTokenSymbol(tokenId) : null))

  useEffect(() => {
    let stale = false
    if (!tokenId) return

    const cached = getCachedTokenSymbol(tokenId)
    if (cached) {
      setResolved(cached)
      return
    }

    if (isCw20) {
      fetchCW20TokenInfo(tokenId).then((result) => {
        if (!stale && result?.symbol) setResolved(result.symbol)
      })
    }

    return () => {
      stale = true
    }
  }, [tokenId, isCw20])

  if (!tokenId) {
    return { displayLabel: '--', symbol: '', addressForBlockie: undefined, logoURI: undefined }
  }

  const chainSymbol = resolved ?? (isAddressLike(tokenId) ? shortenAddress(tokenId) : tokenId)
  const symbol = indexerMeta?.symbol?.trim() || chainSymbol
  const addressForBlockie = isCw20 ? tokenId : undefined
  const logoURI = indexerMeta?.logo_url?.trim() || (info ? getTokenLogoURI(info) : undefined) || undefined

  return { displayLabel: symbol, symbol, addressForBlockie, logoURI }
}
