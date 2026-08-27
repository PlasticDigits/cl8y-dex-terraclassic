import type { IndexerPair, PairInfo } from '@/types'
import { assetInfoLabel } from '@/types'
import {
  CL8Y_TOKEN_ADDRESS,
  LUNC_C_TOKEN_ADDRESS,
  SOFT_LAUNCH_MINTABLE_TOKENS,
  UST1_TOKEN_ADDRESS,
  USTC_C_TOKEN_ADDRESS,
  VFDUSD_TOKEN_ADDRESS,
} from '@/utils/constants'
import { getDecimals } from '@/utils/formatAmount'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { lookupByTokenId } from '@/utils/tokenRegistry'
import { isCustcLeg, isUst1Leg } from '@/utils/tradePairDisplayOrientation'
import { MAINNET_UST1_CUSTC_PAIR_ADDRESS } from '@/utils/ust1SecondaryMarket'

/**
 * Soft-launch + LocalTerra faucet/test tickers (GitLab #534 / #562).
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

/**
 * Hardcoded columbus-5 gem CW20s (GitLab #562). Hide-by-address must work even when
 * Coolify drops `VITE_TOKEN_*` (QUARTZ/PEARL were never in that env list).
 */
export const COLUMBUS5_GEM_ADDRESSES = new Set(
  [
    'terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94', // EMBER
    'terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena', // CORAL
    'terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr', // JADE
    'terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9', // ONYX
    'terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc', // RUBY
    'terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme', // TOPAZ
    'terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z', // QUARTZ
    'terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs', // PEARL
  ].map((addr) => addr.toLowerCase())
)

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

/**
 * Production retail discovery shows gems iff LocalTerra/testnet **or** the
 * Coolify build-arg `VITE_SHOW_TEST_TOKENS=true` (QA). Not a runtime query param (X8).
 * **P562-1**
 */
export function retailExposeTestTokens(): boolean {
  if (import.meta.env.VITE_SHOW_TEST_TOKENS === 'true') return true
  const network = import.meta.env.VITE_NETWORK || 'local'
  return network !== 'mainnet'
}

function isKnownGemAddress(tokenId: string): boolean {
  const lower = tokenId.trim().toLowerCase()
  if (!lower) return false
  if (COLUMBUS5_GEM_ADDRESSES.has(lower)) return true
  return SOFT_LAUNCH_MINTABLE_TOKENS.some((t) => t.address.toLowerCase() === lower)
}

const ECONOMIC_ENV_ADDRESSES = new Set(
  [CL8Y_TOKEN_ADDRESS, LUNC_C_TOKEN_ADDRESS, UST1_TOKEN_ADDRESS, USTC_C_TOKEN_ADDRESS, VFDUSD_TOKEN_ADDRESS]
    .filter(Boolean)
    .map((addr) => addr.toLowerCase())
)

/**
 * Listed economic / wrap / hub ids must never be classified as gems (**U6** / **P534-8** / **X1**).
 * Address in the gem set still wins even if `token_info.symbol` is spoofed as UST1.
 */
export function isEconomicHubTokenId(tokenId: string): boolean {
  if (!tokenId?.trim()) return false
  const lower = tokenId.trim().toLowerCase()
  if (lower === 'uluna' || lower === 'uusd') return true
  if (ECONOMIC_ENV_ADDRESSES.has(lower)) return true
  if (HUB_RANK[canonicalPairSymbol(tokenId)] != null) return true
  if (HUB_RANK[canonicalPairSymbol(getTokenDisplaySymbol(tokenId))] != null) return true
  return lookupByTokenId(tokenId) != null
}

/** True when the id/symbol is a faucet gem (hardcoded columbus-5 addrs, env mintables, or gem ticker). */
export function isGemTokenId(tokenId: string): boolean {
  if (!tokenId?.trim()) return false
  if (isKnownGemAddress(tokenId)) return true
  if (isEconomicHubTokenId(tokenId)) return false
  if (isGemSymbol(tokenId)) return true
  if (isGemSymbol(getTokenDisplaySymbol(tokenId))) return true
  return false
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

export function isRetailHiddenTestToken(tokenId: string): boolean {
  return !retailExposeTestTokens() && isGemTokenId(tokenId)
}

export function isRetailHiddenTestPair(
  symbol0: string,
  symbol1: string,
  tokenId0?: string,
  tokenId1?: string
): boolean {
  return !retailExposeTestTokens() && isTestPair(symbol0, symbol1, tokenId0, tokenId1)
}

/** Omit gems from production browse. LocalTerra / `VITE_SHOW_TEST_TOKENS=true` is a no-op. */
export function filterRetailDiscoveryTokens(tokens: readonly string[]): string[] {
  if (retailExposeTestTokens()) return [...tokens]
  return tokens.filter((t) => !isGemTokenId(t))
}

export function filterRetailDiscoveryPairInfos(pairs: readonly PairInfo[]): PairInfo[] {
  if (retailExposeTestTokens()) return [...pairs]
  return pairs.filter((pair) => {
    const legs = pairInfoToCatalogLegs(pair)
    return !isTestPair(legs.symbol0, legs.symbol1, legs.tokenId0, legs.tokenId1)
  })
}

export function filterRetailDiscoveryIndexerPairs(pairs: readonly IndexerPair[]): IndexerPair[] {
  if (retailExposeTestTokens()) return [...pairs]
  return pairs.filter((pair) => {
    const legs = indexerPairToCatalogLegs(pair)
    return !isTestPair(legs.symbol0, legs.symbol1, legs.tokenId0, legs.tokenId1)
  })
}

/**
 * Production BFS / hybrid quote must not bridge economic tokens through gems (**P562-6**).
 * Exit hatch: if either endpoint is a gem, gem hops stay allowed.
 */
export function shouldRejectGemBridgeQuote(fromToken: string, toToken: string, hopTokens: readonly string[]): boolean {
  if (retailExposeTestTokens()) return false
  if (isGemTokenId(fromToken) || isGemTokenId(toToken)) return false
  return hopTokens.some((t) => isGemTokenId(t))
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

/** First factory pair for bare `/trade` auto-pick (P534-5 / P562-4). */
export function firstCatalogPairAddress(pairs: PairInfo[]): string | undefined {
  return sortPairInfosByCatalog(filterRetailDiscoveryPairInfos(pairs))[0]?.contract_addr
}

export function isUst1CustcIndexerPair(pair: IndexerPair): boolean {
  const a0 = {
    symbol: pair.asset_0.symbol,
    contractAddr: pair.asset_0.contract_addr,
    denom: pair.asset_0.denom,
  }
  const a1 = {
    symbol: pair.asset_1.symbol,
    contractAddr: pair.asset_1.contract_addr,
    denom: pair.asset_1.denom,
  }
  return (isUst1Leg(a0) && isCustcLeg(a1)) || (isUst1Leg(a1) && isCustcLeg(a0))
}

/** First UST1/cUSTC pair in a retail-filtered list. Does not change `/trade` pick. */
export function firstUst1CustcPairAddress(pairs: readonly IndexerPair[]): string | undefined {
  return filterRetailDiscoveryIndexerPairs(pairs).find(isUst1CustcIndexerPair)?.pair_address
}

/** First catalog-sorted economic (or exposed-gem) indexer pair — Charts hero fallback. */
export function firstEconomicIndexerPairAddress(pairs: readonly IndexerPair[]): string | undefined {
  return sortIndexerPairsByCatalog(filterRetailDiscoveryIndexerPairs(pairs))[0]?.pair_address
}

/**
 * Bare `/charts` hero (#680): UST1/cUSTC when listed, else columbus-5 pin on mainnet,
 * else first economic catalog pair. Never invent a LocalTerra address.
 */
export function resolveChartsHeroPairAddress(
  pairs: readonly IndexerPair[],
  network: string = import.meta.env.VITE_NETWORK || 'local'
): string | undefined {
  const listed = firstUst1CustcPairAddress(pairs)
  if (listed) return listed
  if (network === 'mainnet') return MAINNET_UST1_CUSTC_PAIR_ADDRESS
  return firstEconomicIndexerPairAddress(pairs)
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

/** Production Swap pay/receive defaults: first two non-gem ids in factory/wrap order (P562-4). */
export function defaultRetailSwapTokenPair(tokenIds: readonly string[]): [string, string] | undefined {
  const tokens = filterRetailDiscoveryTokens(tokenIds)
  if (tokens.length < 2) return undefined
  return [tokens[0], tokens[1]]
}
