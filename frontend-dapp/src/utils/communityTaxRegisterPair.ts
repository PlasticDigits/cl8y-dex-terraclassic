/**
 * Autoregister + Manage catch-up for community-tax factory pairs (GitLab #633).
 *
 * Register is add-only and factory-gated on-chain (**T592-9**). This module never
 * offers Terraport / GDEX addrs. Do not dump the token into Swap defaults.
 */

import { getCommunityTokens, getHubPrices, getTokenPairs } from '@/services/indexer/client'
import { getPair } from '@/services/terraclassic/factory'
import { getPairInfo, getPool } from '@/services/terraclassic/pair'
import { getChainContractInfo } from '@/services/terraclassic/queries'
import { queryCommunityTaxIsExempt, registerListedPair } from '@/services/terraclassic/communityTaxToken'
import { COMMUNITY_TAX_CODE_ID, UST1_TOKEN_ADDRESS } from '@/utils/constants'
import { tokenAssetInfo, type Asset, type AssetInfo, type PairInfo } from '@/types'
import type { IndexerCommunityTaxSnapshot, IndexerHubPricesResponse, IndexerPair } from '@/types'

export const COMMUNITY_TAX_REGISTER_ALERT_COPY =
  'This market is not collecting buy/sell tax yet. Register the largest CL8Y pool so everyday trades use the listed-pair tax.'

export type UnregisteredFactoryPair = {
  pair: string
  symbols: [string, string]
  usdTvl: number | null
  taxReserve: bigint
  otherReserve: bigint
}

export function isCommunityTaxCodeId(codeId: number, pin = COMMUNITY_TAX_CODE_ID): boolean {
  return pin > 0 && codeId === pin
}

export function shortPairAddr(addr: string): string {
  const a = addr.trim()
  if (a.length <= 16) return a
  return `${a.slice(0, 8)}…${a.slice(-6)}`
}

export function compareUnregisteredPairLp(a: UnregisteredFactoryPair, b: UnregisteredFactoryPair): number {
  if (a.usdTvl != null && b.usdTvl != null && a.usdTvl !== b.usdTvl) {
    return b.usdTvl - a.usdTvl
  }
  if (a.taxReserve !== b.taxReserve) return a.taxReserve > b.taxReserve ? -1 : 1
  if (a.otherReserve !== b.otherReserve) return a.otherReserve > b.otherReserve ? -1 : 1
  return a.pair.localeCompare(b.pair)
}

export function pickHighestLpUnregistered(pairs: UnregisteredFactoryPair[]): UnregisteredFactoryPair | null {
  if (pairs.length === 0) return null
  return [...pairs].sort(compareUnregisteredPairLp)[0] ?? null
}

export function registerLargestPoolLabel(target: UnregisteredFactoryPair): string {
  return `Register largest pool · ${target.symbols[0]}/${target.symbols[1]} · ${shortPairAddr(target.pair)}`
}

export function otherManagedTokensNeedingRegister(
  currentToken: string,
  catalog: IndexerCommunityTaxSnapshot[],
  needing: ReadonlySet<string>
): { address: string; symbol: string }[] {
  const cur = currentToken.trim().toLowerCase()
  return catalog
    .filter((t) => t.contract_address.trim().toLowerCase() !== cur && needing.has(t.contract_address.toLowerCase()))
    .map((t) => ({
      address: t.contract_address,
      symbol: (t.symbol || t.name || shortPairAddr(t.contract_address)).trim(),
    }))
}

function assetTokenAddr(info: AssetInfo): string | null {
  return 'token' in info ? info.token.contract_addr : null
}

function reserveForToken(assets: [Asset, Asset], token: string): bigint {
  const want = token.trim().toLowerCase()
  for (const a of assets) {
    const addr = assetTokenAddr(a.info)
    if (addr && addr.toLowerCase() === want) {
      try {
        return BigInt(a.amount)
      } catch {
        return 0n
      }
    }
  }
  return 0n
}

function hubUsdByAddress(hubs: IndexerHubPricesResponse | null, addr: string): number | null {
  if (!hubs) return null
  const want = addr.trim().toLowerCase()
  for (const p of hubs.prices ?? []) {
    if (p.asset_address?.trim().toLowerCase() === want && p.price_usd) {
      const n = Number(p.price_usd)
      if (Number.isFinite(n) && n > 0) return n
    }
    if (p.ticker === 'ust1' && UST1_TOKEN_ADDRESS && UST1_TOKEN_ADDRESS.toLowerCase() === want && p.price_usd) {
      const n = Number(p.price_usd)
      if (Number.isFinite(n) && n > 0) return n
    }
  }
  return null
}

function usdTvl(assets: [Asset, Asset], hubs: IndexerHubPricesResponse | null): number | null {
  if (!hubs) return null
  let sum = 0
  for (const a of assets) {
    const addr = assetTokenAddr(a.info)
    if (!addr) return null
    const px = hubUsdByAddress(hubs, addr)
    if (px == null) return null
    try {
      sum += Number(BigInt(a.amount)) * px
    } catch {
      return null
    }
  }
  return sum
}

export async function verifyFactoryListedPair(pairAddr: string): Promise<PairInfo | null> {
  try {
    const info = await getPairInfo(pairAddr)
    if (info.contract_addr.trim().toLowerCase() !== pairAddr.trim().toLowerCase()) return null
    const factoryPair = await getPair(info.asset_infos)
    if (factoryPair.contract_addr.trim().toLowerCase() !== pairAddr.trim().toLowerCase()) return null
    return factoryPair
  } catch {
    return null
  }
}

export async function loadUnregisteredFactoryPairs(
  token: string,
  indexerPairs?: IndexerPair[]
): Promise<UnregisteredFactoryPair[]> {
  const rows = indexerPairs ?? (await getTokenPairs(token).catch(() => []))
  const hubs = await getHubPrices().catch(() => null)
  const out: UnregisteredFactoryPair[] = []
  for (const row of rows) {
    const pairAddr = row.pair_address?.trim()
    if (!pairAddr) continue
    const verified = await verifyFactoryListedPair(pairAddr)
    if (!verified) continue
    const exempt = await queryCommunityTaxIsExempt(token, pairAddr).catch(() => null)
    if (exempt?.protocol) continue
    const pool = await getPool(pairAddr).catch(() => null)
    if (!pool) continue
    const taxReserve = reserveForToken(pool.assets, token)
    const other = pool.assets.find((a) => {
      const addr = assetTokenAddr(a.info)
      return !addr || addr.toLowerCase() !== token.trim().toLowerCase()
    })
    let otherReserve = 0n
    try {
      otherReserve = other ? BigInt(other.amount) : 0n
    } catch {
      otherReserve = 0n
    }
    const sym0 = row.asset_0?.symbol || 'Token'
    const sym1 = row.asset_1?.symbol || 'Token'
    out.push({
      pair: pairAddr,
      symbols: [sym0, sym1],
      usdTvl: usdTvl(pool.assets, hubs),
      taxReserve,
      otherReserve,
    })
  }
  return out
}

export type RegisterAfterCreateResult = {
  pair: string
  registered: string[]
}

/**
 * B1: after a successful factory CreatePair, register each community-tax side.
 * Honest-only pairs skip the extra execute. Partial failure throws so the page
 * can point at Manage — do not pretend the market is listed.
 */
export async function registerTaxAssetsAfterCreatePair(input: {
  wallet: string
  tokenA: string
  tokenB: string
  taxCodeId?: number
}): Promise<RegisterAfterCreateResult> {
  const pin = input.taxCodeId ?? COMMUNITY_TAX_CODE_ID
  const tokenA = input.tokenA.trim()
  const tokenB = input.tokenB.trim()
  const pairInfo = await getPair([tokenAssetInfo(tokenA), tokenAssetInfo(tokenB)])
  const pair = pairInfo.contract_addr
  const taxTokens: string[] = []
  if (pin > 0) {
    for (const token of [tokenA, tokenB]) {
      const info = await getChainContractInfo(token).catch(() => null)
      if (info && isCommunityTaxCodeId(info.code_id, pin)) taxTokens.push(token)
    }
  }
  const registered: string[] = []
  const errors: string[] = []
  for (const token of taxTokens) {
    try {
      await registerListedPair(input.wallet, token, pair)
      registered.push(token)
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      errors.push(msg)
    }
  }
  if (errors.length > 0) {
    throw new Error(
      `The pair was created but buy/sell tax is not on yet. Open Manage to register the pool. ${errors.join(' ')}`
    )
  }
  return { pair, registered }
}

export async function tokensNeedingRegisterForManager(
  manager: string,
  currentToken: string
): Promise<{ address: string; symbol: string }[]> {
  const list = await getCommunityTokens({ manager }).catch(() => ({ items: [] as IndexerCommunityTaxSnapshot[] }))
  const needing = new Set<string>()
  for (const item of list.items ?? []) {
    const addr = item.contract_address?.trim()
    if (!addr || addr.toLowerCase() === currentToken.trim().toLowerCase()) continue
    const unreg = await loadUnregisteredFactoryPairs(addr).catch(() => [])
    if (unreg.length > 0) needing.add(addr.toLowerCase())
  }
  return otherManagedTokensNeedingRegister(currentToken, list.items ?? [], needing)
}
