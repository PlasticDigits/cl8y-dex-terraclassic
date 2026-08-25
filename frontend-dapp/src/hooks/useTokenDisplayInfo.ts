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
import { resolveTrustedTokenLogoUrl } from '@/utils/tokenLogoAllowlist'
import { registryProductSymbol } from '@/utils/tokenRegistry'

export interface TokenDisplayInfo {
  displayLabel: string
  symbol: string
  addressForBlockie: string | undefined
  logoURI: string | undefined
}

/** Match indexer rows by identity, not display symbol (GitLab #630). */
export function indexerTokenForId(tokenId: string, list: IndexerToken[] | undefined) {
  if (!list?.length || !tokenId) return undefined
  const isCw20Id = tokenId.toLowerCase().startsWith('terra1')
  if (isCw20Id) {
    const t = tokenId.toLowerCase()
    return list.find((x) => x.contract_address && x.contract_address.toLowerCase() === t)
  }
  // Native id: denom equality only. Do not bind a CW20 whose address string equals the denom.
  return list.find((x) => x.denom === tokenId && !x.contract_address)
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
  // Registry product tickers (LUNC/USTC/cLUNC/cUSTC/…) beat indexer/on-chain text (#507, #630).
  const productSymbol = registryProductSymbol(tokenId)
  const symbol = productSymbol || indexerMeta?.symbol?.trim() || chainSymbol
  const addressForBlockie = isCw20 ? tokenId : undefined
  const rawLogo = indexerMeta?.logo_url?.trim() || (info ? getTokenLogoURI(info) : undefined) || undefined
  const logoURI = resolveTrustedTokenLogoUrl(rawLogo)

  return { displayLabel: symbol, symbol, addressForBlockie, logoURI }
}
