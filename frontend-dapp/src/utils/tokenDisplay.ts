import { queryContract } from '@/services/terraclassic/queries'
import type { AssetInfo } from '@/types'
import { lookupByTokenId, lookupByAssetInfo } from './tokenRegistry'

interface CW20TokenInfo {
  name: string
  symbol: string
  decimals: number
  total_supply: string
}

const CACHE_KEY = 'cl8y-dex-token-info'

type CachedEntry = { symbol: string; name: string }

function loadCache(): Record<string, CachedEntry> {
  try {
    return JSON.parse(localStorage.getItem(CACHE_KEY) || '{}')
  } catch {
    return {}
  }
}

function saveCache(cache: Record<string, CachedEntry>) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(cache))
  } catch {
    // quota exceeded
  }
}

const inFlightQueries = new Map<string, Promise<CW20TokenInfo | null>>()

export function getCachedTokenSymbol(tokenId: string): string | null {
  const reg = lookupByTokenId(tokenId)
  if (reg) return reg.symbol
  const cache = loadCache()
  return cache[tokenId.toLowerCase()]?.symbol ?? null
}

export function getTokenLogoURI(info: AssetInfo): string | undefined {
  return lookupByAssetInfo(info)?.logoURI
}

export async function fetchCW20TokenInfo(contractAddr: string): Promise<CW20TokenInfo | null> {
  const key = contractAddr.toLowerCase()
  const existing = inFlightQueries.get(key)
  if (existing) return existing

  const promise = queryContract<CW20TokenInfo>(contractAddr, { token_info: {} })
    .then((info) => {
      const cache = loadCache()
      cache[key] = { symbol: info.symbol, name: info.name }
      saveCache(cache)
      inFlightQueries.delete(key)
      return info
    })
    .catch(() => {
      inFlightQueries.delete(key)
      return null
    })

  inFlightQueries.set(key, promise)
  return promise
}

export function getTokenDisplaySymbol(tokenId: string): string {
  if (!tokenId?.trim()) return ''
  const reg = lookupByTokenId(tokenId)
  if (reg) return reg.symbol
  if (tokenId.toLowerCase().startsWith('terra1') && tokenId.length >= 44) {
    const cache = loadCache()
    if (cache[tokenId.toLowerCase()]?.symbol) return cache[tokenId.toLowerCase()].symbol
    return shortenAddress(tokenId)
  }
  return tokenId
}

export function shortenAddress(addr: string): string {
  if (addr.length <= 16) return addr
  return `${addr.slice(0, 8)}…${addr.slice(-6)}`
}

export function isAddressLike(s: string): boolean {
  return (s.startsWith('terra1') && s.length >= 44) || (s.startsWith('0x') && s.length >= 42)
}

export function getAddressForBlockie(info: AssetInfo): string | undefined {
  if ('token' in info) return info.token.contract_addr
  return undefined
}
