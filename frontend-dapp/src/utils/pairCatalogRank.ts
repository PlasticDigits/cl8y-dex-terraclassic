import type { IndexerPair, PairInfo } from '@/types'
import { assetInfoLabel } from '@/types'
import { SOFT_LAUNCH_MINTABLE_TOKENS } from '@/utils/constants'
import { getDecimals } from '@/utils/formatAmount'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'

/**
 * Soft-launch + LocalTerra faucet/test tickers (GitLab #534).
 * Economic = both legs resolve outside this set (cLUNC, cUSTC, UST1, USTR, CL8Y, …).
 */
export const GEM_SYMBOLS = new Set([
  'EMBER',
  'CORAL',
  'JADE',
  'ONYX',
  'RUBY',
  'TOPAZ',
  'QUARTZ',
  'PEARL',
  'OPAL',
  'COBALT',
  'SLATE',
  'AMBER',
  'IRON',
])

/** Lower rank = listed earlier. UST1 is the launch hub so its markets sit together. */
const HUB_RANK: Record<string, number> = {
  UST1: 0,
  CLUNC: 1,
  CUSTC: 2,
  USTR: 3,
  CL8Y: 4,
  VFDUSD: 5,
}

export type PairCatalogVolume = {
  raw?: string | null
  quoteDecimals?: number
}

export type PairCatalogLegs = {
  symbol0: string
  symbol1: string
  tokenId0?: string
  tokenId1?: string
  volume?: PairCatalogVolume
  address?: string
}

/** Collapse display / native / wrap aliases to a hub or gem key. */
export function canonicalPairSymbol(raw: string): string {
  const trimmed = raw.trim()
  if (!trimmed) return ''
  const upper = trimmed.toUpperCase()
  if (upper === 'ULUNA' || upper === 'LUNC' || upper === 'CLUNC' || upper === 'LUNC-C' || trimmed === 'cLUNC') {
    return 'CLUNC'
  }
  if (upper === 'UUSD' || upper === 'USTC' || upper === 'CUSTC' || upper === 'USTC-C' || trimmed === 'cUSTC') {
    return 'CUSTC'
  }
  if (upper === 'TCL8Y') return 'CL8Y'
  if (upper === 'VFDUSD') return 'VFDUSD'
  return upper
}

export function isGemSymbol(symbol: string): boolean {
  return GEM_SYMBOLS.has(canonicalPairSymbol(symbol))
}

/** True when the id/symbol is a faucet gem (symbol set, cache/registry, or mintable env address). */
export function isGemTokenId(tokenId: string): boolean {
  if (!tokenId?.trim()) return false
  if (isGemSymbol(tokenId)) return true
  if (isGemSymbol(getTokenDisplaySymbol(tokenId))) return true
  const lower = tokenId.toLowerCase()
  return SOFT_LAUNCH_MINTABLE_TOKENS.some((t) => t.address.toLowerCase() === lower)
}

/**
 * Economic pair = both legs are real tokens. A gem on either side is a test pair (P534-1).
 */
export function isTestPair(symbol0: string, symbol1: string, tokenId0?: string, tokenId1?: string): boolean {
  return (
    isGemSymbol(symbol0) ||
    isGemSymbol(symbol1) ||
    (!!tokenId0 && isGemTokenId(tokenId0)) ||
    (!!tokenId1 && isGemTokenId(tokenId1))
  )
}

function hubRankForLeg(symbol: string): number | undefined {
  return HUB_RANK[canonicalPairSymbol(symbol)]
}

function pairHubRank(symbol0: string, symbol1: string): number {
  const ranks = [hubRankForLeg(symbol0), hubRankForLeg(symbol1)].filter((n): n is number => n != null)
  return ranks.length > 0 ? Math.min(...ranks) : 100
}

function otherHubSymbol(symbol0: string, symbol1: string): string {
  const r0 = hubRankForLeg(symbol0)
  const r1 = hubRankForLeg(symbol1)
  if (r0 != null && r1 != null) {
    return r0 <= r1 ? canonicalPairSymbol(symbol1) : canonicalPairSymbol(symbol0)
  }
  if (r0 != null) return canonicalPairSymbol(symbol1)
  if (r1 != null) return canonicalPairSymbol(symbol0)
  const a = canonicalPairSymbol(symbol0)
  const b = canonicalPairSymbol(symbol1)
  return a <= b ? a : b
}

function parseRawVolume(raw?: string | null): bigint {
  if (!raw) return 0n
  try {
    const n = BigInt(raw)
    return n < 0n ? 0n : n
  } catch {
    return 0n
  }
}

function clampDecimals(decimals: number | undefined): number {
  if (decimals == null || !Number.isFinite(decimals)) return 6
  return Math.max(0, Math.min(36, Math.trunc(decimals)))
}

/** Compare human quote volumes `raw / 10^decimals` descending (mixed 6/18-dec safe). */
export function compareHumanQuoteVolumeDesc(
  aRaw: string | null | undefined,
  aDecimals: number | undefined,
  bRaw: string | null | undefined,
  bDecimals: number | undefined
): number {
  const a = parseRawVolume(aRaw)
  const b = parseRawVolume(bRaw)
  const da = BigInt(clampDecimals(aDecimals))
  const db = BigInt(clampDecimals(bDecimals))
  const left = a * 10n ** db
  const right = b * 10n ** da
  if (left > right) return -1
  if (left < right) return 1
  return 0
}

export function comparePairCatalog(a: PairCatalogLegs, b: PairCatalogLegs): number {
  const aTest = isTestPair(a.symbol0, a.symbol1, a.tokenId0, a.tokenId1) ? 1 : 0
  const bTest = isTestPair(b.symbol0, b.symbol1, b.tokenId0, b.tokenId1) ? 1 : 0
  if (aTest !== bTest) return aTest - bTest

  const hub = pairHubRank(a.symbol0, a.symbol1) - pairHubRank(b.symbol0, b.symbol1)
  if (hub !== 0) return hub

  const vol = compareHumanQuoteVolumeDesc(
    a.volume?.raw,
    a.volume?.quoteDecimals,
    b.volume?.raw,
    b.volume?.quoteDecimals
  )
  if (vol !== 0) return vol

  const other = otherHubSymbol(a.symbol0, a.symbol1).localeCompare(otherHubSymbol(b.symbol0, b.symbol1))
  if (other !== 0) return other

  const labelA = `${canonicalPairSymbol(a.symbol0)}/${canonicalPairSymbol(a.symbol1)}`
  const labelB = `${canonicalPairSymbol(b.symbol0)}/${canonicalPairSymbol(b.symbol1)}`
  const label = labelA.localeCompare(labelB)
  if (label !== 0) return label

  return (a.address ?? '').localeCompare(b.address ?? '')
}

export function pairInfoLegIds(pair: PairInfo): [string, string] {
  return [assetInfoLabel(pair.asset_infos[0]), assetInfoLabel(pair.asset_infos[1])]
}

export function pairInfoLegSymbols(pair: PairInfo): [string, string] {
  const [id0, id1] = pairInfoLegIds(pair)
  return [getTokenDisplaySymbol(id0), getTokenDisplaySymbol(id1)]
}

export function pairInfoToCatalogLegs(pair: PairInfo, volume?: PairCatalogVolume): PairCatalogLegs {
  const [tokenId0, tokenId1] = pairInfoLegIds(pair)
  const [symbol0, symbol1] = pairInfoLegSymbols(pair)
  return {
    symbol0,
    symbol1,
    tokenId0,
    tokenId1,
    volume: volume ?? { quoteDecimals: getDecimals(pair.asset_infos[1]) },
    address: pair.contract_addr,
  }
}

export function indexerPairToCatalogLegs(pair: IndexerPair): PairCatalogLegs {
  return {
    symbol0: pair.asset_0.symbol,
    symbol1: pair.asset_1.symbol,
    tokenId0: pair.asset_0.contract_addr ?? pair.asset_0.denom ?? undefined,
    tokenId1: pair.asset_1.contract_addr ?? pair.asset_1.denom ?? undefined,
    volume: { raw: pair.volume_quote_24h, quoteDecimals: pair.asset_1.decimals },
    address: pair.pair_address,
  }
}

export function sortPairInfosByCatalog(
  pairs: PairInfo[],
  volumeByAddress?: Map<string, PairCatalogVolume>
): PairInfo[] {
  return [...pairs].sort((a, b) =>
    comparePairCatalog(
      pairInfoToCatalogLegs(a, volumeByAddress?.get(a.contract_addr)),
      pairInfoToCatalogLegs(b, volumeByAddress?.get(b.contract_addr))
    )
  )
}

export function sortIndexerPairsByCatalog(pairs: IndexerPair[]): IndexerPair[] {
  return [...pairs].sort((a, b) => comparePairCatalog(indexerPairToCatalogLegs(a), indexerPairToCatalogLegs(b)))
}

/** First factory pair for bare `/trade` auto-pick (P534-5). */
export function firstCatalogPairAddress(pairs: PairInfo[]): string | undefined {
  return sortPairInfosByCatalog(pairs)[0]?.contract_addr
}

export function compareTokenCatalog(tokenIdA: string, tokenIdB: string): number {
  const aGem = isGemTokenId(tokenIdA) ? 1 : 0
  const bGem = isGemTokenId(tokenIdB) ? 1 : 0
  if (aGem !== bGem) return aGem - bGem
  const sa = getTokenDisplaySymbol(tokenIdA)
  const sb = getTokenDisplaySymbol(tokenIdB)
  const hub = (hubRankForLeg(sa) ?? 100) - (hubRankForLeg(sb) ?? 100)
  if (hub !== 0) return hub
  const cmp = sa.toLowerCase().localeCompare(sb.toLowerCase())
  if (cmp !== 0) return cmp
  return tokenIdA.localeCompare(tokenIdB)
}
