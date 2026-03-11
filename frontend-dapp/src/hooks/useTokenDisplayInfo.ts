import { useEffect, useState } from 'react'
import type { AssetInfo } from '@/types'
import {
  getCachedTokenSymbol,
  fetchCW20TokenInfo,
  shortenAddress,
  isAddressLike,
} from '@/utils/tokenDisplay'

export interface TokenDisplayInfo {
  displayLabel: string
  symbol: string
  addressForBlockie: string | undefined
}

export function useTokenDisplayInfo(info: AssetInfo | null): TokenDisplayInfo {
  const tokenId = info
    ? 'token' in info
      ? info.token.contract_addr
      : info.native_token.denom
    : ''
  const isCw20 = !!info && 'token' in info

  const [resolved, setResolved] = useState<string | null>(() =>
    tokenId ? getCachedTokenSymbol(tokenId) : null
  )

  useEffect(() => {
    if (!tokenId) return

    const cached = getCachedTokenSymbol(tokenId)
    if (cached) {
      setResolved(cached)
      return
    }

    if (isCw20) {
      fetchCW20TokenInfo(tokenId).then((info) => {
        if (info?.symbol) setResolved(info.symbol)
      })
    }
  }, [tokenId, isCw20])

  if (!tokenId) {
    return { displayLabel: '--', symbol: '', addressForBlockie: undefined }
  }

  const symbol = resolved ?? (isAddressLike(tokenId) ? shortenAddress(tokenId) : tokenId)
  const addressForBlockie = isCw20 ? tokenId : undefined

  return { displayLabel: symbol, symbol, addressForBlockie }
}
