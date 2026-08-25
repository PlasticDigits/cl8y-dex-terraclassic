/**
 * Migrate Token venue inventory (GitLab #634 / **M634**).
 *
 * Discover CL8Y factory pairs vs other-DEX (Terraport static + factory probe).
 * Never `RegisterListedPair` Terraport / GDEX. Token admin cannot Refresh.
 * Ranking for register is CL8Y-only — do not reuse #633 highest-LP across venues.
 */

import { getTokenPairs } from '@/services/indexer/client'
import { probePairCodeIdFreeze } from '@/services/terraclassic/assetCodeIdFreeze'
import { getAllPairsPaginated, getPair, isCodeIdWhitelisted } from '@/services/terraclassic/factory'
import { getPool } from '@/services/terraclassic/pair'
import { queryCommunityTaxIsExempt, registerListedPair } from '@/services/terraclassic/communityTaxToken'
import { getChainContractInfo, queryContract } from '@/services/terraclassic/queries'
import { verifyFactoryListedPair } from '@/utils/communityTaxRegisterPair'
import { FACTORY_CONTRACT_ADDRESS, UST1_TOKEN_ADDRESS } from '@/utils/constants'
import { tokenAssetInfo, type AssetInfo, type PairInfo } from '@/types'
import type { IndexerPair } from '@/types'

/** Columbus-5 Terraport factory (`docs/terraport.md`). Not a CL8Y register target. */
export const TERRAPORT_FACTORY_ADDRESS = 'terra1n75fgfc8clsssrm2k0fswgtzsvstdaah7la6sfu96szdu22xta0q57rqqr'

export const ALPHA_TOKEN_ADDRESS = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'
export const ALPHA_TERRAPORT_LUNC_PAIR = 'terra12u7khzrzn05a73xkpq6a5zrcazz2xmqn7lvupmqmca06pgcyt5qsa9e7p6'
export const ALPHA_TERRAPORT_USTC_PAIR = 'terra1jg2vu97ssz2ldn6gztyl4fp9lfdtc23ffr65l4gpvuxw4znkmpxsja5wph'

/** Open 8266 — full bech32 from LCD 2026-08-25 (#636). */
export const OPEN_TOKEN_ADDRESS = 'terra1qz56v6p8ca3hh34wnj5yc3jykmw6jaaal0ukecscq8m9qqtgztnscs74n3'
export const OPEN_TERRAPORT_LUNC_PAIR = 'terra1uxr6m55wxez5csnttz00893zur6pksn54nwlpye0c2pyuyyqp3qqknypyc'
export const OPEN_TOKEN_PREFIX = 'terra1qz56v'
export const OPEN_TOKEN_SUFFIX = 's74n3'
export const OPEN_TERRAPORT_LUNC_PREFIX = 'terra1uxr6m'
export const OPEN_TERRAPORT_LUNC_SUFFIX = 'nypyc'

export const MIGRATE_VENUE_CL8Y_EMPTY = 'No CL8Y pool yet. Create a pair after you migrate.'
export const MIGRATE_VENUE_CL8Y_PAUSE =
  'This CL8Y market pauses until CL8Y governance refreshes the pool. You cannot refresh it.'
export const MIGRATE_VENUE_OTHER_DEX = 'Leave these pools. They stay 1:1. Do not register them here.'
export const MIGRATE_VENUE_GDEX = 'Other DEX pools (if any) keep this address. Do not register them. They stay 1:1.'
export const MIGRATE_VENUE_TERRAPORT_INCOMPLETE =
  'We could not list every other-DEX pool. Leave those pools as they are.'
export const MIGRATE_REGISTER_WAIT = 'Wait until governance refreshes this pool.'
export const MIGRATE_REGISTER_UNLISTED_OTHER =
  'Governance will skip this pool. Create a new CL8Y pair versus a listed token instead.'
export const MIGRATE_REGISTER_READY = 'Register this CL8Y pool'
export const MIGRATE_CREATE_PAIR_HINT = 'New CL8Y pools are registered when you create them.'
export const MIGRATE_SUCCESS_CHECKLIST = [
  'Token address unchanged. You are now the manager.',
  'CL8Y pools wait for governance refresh, or create a new CL8Y pair if none.',
  'After refresh, register the CL8Y pool so buy/sell tax applies. Your manager trades are not taxed.',
  'Other DEX: do nothing. Do not paste those pool addresses into register.',
] as const

export type MigrateVenue = 'cl8y' | 'other_dex'

export type OtherDexSource = 'static' | 'terraport_factory'

export type ListedFlag = boolean | 'unknown'

export type Cl8yVenueRow = {
  venue: 'cl8y'
  pair: string
  symbols: [string, string]
  otherAssetLabel: string
  otherAssetListed: ListedFlag
  frozen: ListedFlag
  registered: ListedFlag
  factoryVerified: true
}

export type OtherDexVenueRow = {
  venue: 'other_dex'
  pair: string
  pairDisplay?: string
  symbols: [string, string]
  source: OtherDexSource
}

export type MigratePairInventory = {
  cl8y: Cl8yVenueRow[]
  otherDex: OtherDexVenueRow[]
  terraportIncomplete: boolean
}

export type RegisterCtaState = 'hidden' | 'ready' | 'wait_refresh' | 'skip_unlisted' | 'already'

export type RegisterCtaContext = {
  postAdopt: boolean
  isManager: boolean
  taxPinMatches: boolean
  adminIsCmm: boolean
}

export type KnownTerraportRow = {
  token?: string
  tokenPrefix?: string
  tokenSuffix?: string
  pair: string
  pairPrefix?: string
  pairSuffix?: string
  pairDisplay?: string
  symbols: [string, string]
  quoteDenom: string
}

export const KNOWN_TERRAPORT_ROWS: readonly KnownTerraportRow[] = [
  {
    token: ALPHA_TOKEN_ADDRESS,
    pair: ALPHA_TERRAPORT_LUNC_PAIR,
    symbols: ['ALPHA', 'LUNC'],
    quoteDenom: 'uluna',
  },
  {
    token: ALPHA_TOKEN_ADDRESS,
    pair: ALPHA_TERRAPORT_USTC_PAIR,
    symbols: ['ALPHA', 'USTC'],
    quoteDenom: 'uusd',
  },
  {
    token: OPEN_TOKEN_ADDRESS,
    tokenPrefix: OPEN_TOKEN_PREFIX,
    tokenSuffix: OPEN_TOKEN_SUFFIX,
    pair: OPEN_TERRAPORT_LUNC_PAIR,
    pairPrefix: OPEN_TERRAPORT_LUNC_PREFIX,
    pairSuffix: OPEN_TERRAPORT_LUNC_SUFFIX,
    symbols: ['Open', 'LUNC'],
    quoteDenom: 'uluna',
  },
]

const TERRAPORT_QUOTE_DENOMS = ['uluna', 'uusd'] as const

export function addrsEqual(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase()
}

export function tokenMatchesKnown(token: string, known: KnownTerraportRow): boolean {
  const t = token.trim().toLowerCase()
  if (known.token && addrsEqual(t, known.token)) return true
  if (known.tokenPrefix && known.tokenSuffix) {
    return t.startsWith(known.tokenPrefix.toLowerCase()) && t.endsWith(known.tokenSuffix.toLowerCase())
  }
  return false
}

export function knownTerraportRowsForToken(token: string): KnownTerraportRow[] {
  return KNOWN_TERRAPORT_ROWS.filter((row) => tokenMatchesKnown(token, row))
}

export function pairContainsToken(assetInfos: [AssetInfo, AssetInfo], token: string): boolean {
  const want = token.trim().toLowerCase()
  return assetInfos.some((info) => 'token' in info && info.token.contract_addr.trim().toLowerCase() === want)
}

export function otherAssetInfo(assetInfos: [AssetInfo, AssetInfo], token: string): AssetInfo {
  const want = token.trim().toLowerCase()
  const other = assetInfos.find((info) => !('token' in info) || info.token.contract_addr.trim().toLowerCase() !== want)
  return other ?? assetInfos[1]
}

export function labelAssetInfo(info: AssetInfo): string {
  if ('native_token' in info) {
    if (info.native_token.denom === 'uluna') return 'LUNC'
    if (info.native_token.denom === 'uusd') return 'USTC'
    return info.native_token.denom
  }
  return info.token.contract_addr
}

export function isPairMissingError(err: unknown): boolean {
  const s = err instanceof Error ? err.message : String(err)
  return /not found|no pair|unknown pair|does not exist|no data/i.test(s)
}

/**
 * Register CTA is CL8Y + factory-verified + post-adopt + tax pin + CMM admin + manager.
 * Frozen → wait. Unlisted other asset → skip (do not tell them to refresh).
 * Terraport / GDEX never reach this helper with venue cl8y.
 */
export function registerCtaState(row: Cl8yVenueRow, ctx: RegisterCtaContext): RegisterCtaState {
  if (!ctx.postAdopt || !ctx.isManager || !ctx.taxPinMatches || !ctx.adminIsCmm) return 'hidden'
  if (!row.factoryVerified) return 'hidden'
  if (row.otherAssetListed === false) return 'skip_unlisted'
  if (row.registered === true) return 'already'
  if (row.frozen === true) return 'wait_refresh'
  if (row.frozen === 'unknown') return 'wait_refresh'
  return 'ready'
}

export function overlayKnownTerraportRows(token: string, discovered: OtherDexVenueRow[]): OtherDexVenueRow[] {
  const out = [...discovered]
  for (const known of knownTerraportRowsForToken(token)) {
    const already = out.some((row) => {
      if (known.pair && row.pair && addrsEqual(row.pair, known.pair)) return true
      if (known.pairPrefix && known.pairSuffix && row.pair) {
        const p = row.pair.toLowerCase()
        return p.startsWith(known.pairPrefix.toLowerCase()) && p.endsWith(known.pairSuffix.toLowerCase())
      }
      return false
    })
    if (already) continue
    out.push({
      venue: 'other_dex',
      pair: known.pair,
      pairDisplay: known.pairDisplay,
      symbols: known.symbols,
      source: 'static',
    })
  }
  return out
}

export function buildGovernanceTicket(token: string, rows: Cl8yVenueRow[]): string {
  const lines = [`Token: ${token.trim()}`]
  if (rows.length === 0) {
    lines.push('No CL8Y pool found.')
    return lines.join('\n')
  }
  for (const row of rows) {
    lines.push(`CL8Y pool: ${row.pair} (${row.symbols[0]}/${row.symbols[1]})`)
    lines.push(`Other side: ${row.otherAssetLabel}`)
    if (row.otherAssetListed === false) {
      lines.push('Skip Refresh: other asset is not factory-listed. Create a new CL8Y pair instead.')
    } else {
      lines.push('RefreshPair after 11619 adopt')
    }
  }
  return lines.join('\n')
}

export async function queryFactoryPairAt(
  factory: string,
  assetInfos: [AssetInfo, AssetInfo]
): Promise<PairInfo | null> {
  const resp = await queryContract<PairInfo | { pair: PairInfo }>(factory, {
    pair: { asset_infos: assetInfos },
  })
  if (resp && typeof resp === 'object' && 'pair' in resp && resp.pair?.contract_addr) {
    return resp.pair
  }
  if (resp && typeof resp === 'object' && 'contract_addr' in resp && resp.contract_addr) {
    return resp as PairInfo
  }
  return null
}

export async function queryFactoryPairEitherOrder(
  factory: string,
  token: string,
  quote: AssetInfo
): Promise<PairInfo | null> {
  const tokenInfo = tokenAssetInfo(token)
  let transport: unknown = null
  for (const infos of [
    [tokenInfo, quote],
    [quote, tokenInfo],
  ] as [AssetInfo, AssetInfo][]) {
    try {
      const pair = await queryFactoryPairAt(factory, infos)
      if (pair) return pair
    } catch (err) {
      if (isPairMissingError(err)) continue
      transport = err
    }
  }
  if (transport) throw transport
  return null
}

function indexerSymbols(row: IndexerPair, token: string): [string, string] {
  const want = token.trim().toLowerCase()
  const a0 = row.asset_0
  const a1 = row.asset_1
  const tokenIs0 = a0.contract_addr?.trim().toLowerCase() === want
  const tokenSym = (tokenIs0 ? a0.symbol : a1.symbol) || 'Token'
  const otherSym = (tokenIs0 ? a1.symbol : a0.symbol) || 'Token'
  return [tokenSym, otherSym]
}

async function probeOtherAssetListed(info: AssetInfo): Promise<ListedFlag> {
  if ('native_token' in info) return true
  try {
    const ci = await getChainContractInfo(info.token.contract_addr)
    const wl = await isCodeIdWhitelisted(ci.code_id)
    return wl.whitelisted === true
  } catch {
    return 'unknown'
  }
}

function cl8yQuoteAssets(): AssetInfo[] {
  const quotes: AssetInfo[] = [{ native_token: { denom: 'uluna' } }, { native_token: { denom: 'uusd' } }]
  if (UST1_TOKEN_ADDRESS) quotes.push(tokenAssetInfo(UST1_TOKEN_ADDRESS))
  return quotes
}

async function discoverCl8yCandidates(token: string): Promise<{ pair: string; symbols: [string, string] }[]> {
  const byPair = new Map<string, { pair: string; symbols: [string, string] }>()
  const add = (pair: string, symbols: [string, string]) => {
    const key = pair.trim().toLowerCase()
    if (!key || byPair.has(key)) return
    byPair.set(key, { pair: pair.trim(), symbols })
  }

  const indexerRows = await getTokenPairs(token).catch(() => null)
  if (indexerRows) {
    for (const row of indexerRows) {
      if (row.pair_address) add(row.pair_address, indexerSymbols(row, token))
    }
  }

  if (!indexerRows || indexerRows.length === 0) {
    if (FACTORY_CONTRACT_ADDRESS) {
      const all = await getAllPairsPaginated(200).catch(() => ({ pairs: [] as PairInfo[] }))
      for (const p of all.pairs ?? []) {
        if (pairContainsToken(p.asset_infos, token)) {
          const other = otherAssetInfo(p.asset_infos, token)
          add(p.contract_addr, ['Token', labelAssetInfo(other)])
        }
      }
      for (const quote of cl8yQuoteAssets()) {
        try {
          const found = await getPair([tokenAssetInfo(token), quote]).catch(() =>
            getPair([quote, tokenAssetInfo(token)]).catch(() => null)
          )
          if (found && pairContainsToken(found.asset_infos, token)) {
            const other = otherAssetInfo(found.asset_infos, token)
            add(found.contract_addr, ['Token', labelAssetInfo(other)])
          }
        } catch {
          // probe miss is fine
        }
      }
    }
  }

  return [...byPair.values()]
}

async function enrichCl8yRow(
  token: string,
  candidate: { pair: string; symbols: [string, string] },
  postAdopt: boolean
): Promise<Cl8yVenueRow | null> {
  const verified = await verifyFactoryListedPair(candidate.pair)
  if (!verified || !pairContainsToken(verified.asset_infos, token)) return null
  const other = otherAssetInfo(verified.asset_infos, token)
  let symbols = candidate.symbols
  const pool = await getPool(candidate.pair).catch(() => null)
  if (pool) {
    const otherLabel = labelAssetInfo(other)
    symbols = [symbols[0] === 'Token' ? 'Token' : symbols[0], otherLabel]
  }
  let frozen: ListedFlag = 'unknown'
  let registered: ListedFlag = 'unknown'
  if (postAdopt) {
    const probe = await probePairCodeIdFreeze(candidate.pair).catch(() => null)
    frozen = probe ? probe.frozen : 'unknown'
    const exempt = await queryCommunityTaxIsExempt(token, candidate.pair).catch(() => null)
    registered = exempt ? exempt.protocol === true : 'unknown'
  }
  return {
    venue: 'cl8y',
    pair: verified.contract_addr,
    symbols,
    otherAssetLabel: labelAssetInfo(other),
    otherAssetListed: await probeOtherAssetListed(other),
    frozen,
    registered,
    factoryVerified: true,
  }
}

async function discoverTerraport(token: string): Promise<{ rows: OtherDexVenueRow[]; incomplete: boolean }> {
  const found: OtherDexVenueRow[] = []
  let incomplete = false
  for (const denom of TERRAPORT_QUOTE_DENOMS) {
    try {
      const pair = await queryFactoryPairEitherOrder(TERRAPORT_FACTORY_ADDRESS, token, {
        native_token: { denom },
      })
      if (!pair?.contract_addr) continue
      const other = otherAssetInfo(pair.asset_infos, token)
      found.push({
        venue: 'other_dex',
        pair: pair.contract_addr,
        symbols: ['Token', labelAssetInfo(other)],
        source: 'terraport_factory',
      })
    } catch {
      incomplete = true
    }
  }
  return { rows: overlayKnownTerraportRows(token, found), incomplete }
}

export async function loadMigratePairInventory(
  token: string,
  opts?: { postAdopt?: boolean }
): Promise<MigratePairInventory> {
  const addr = token.trim()
  const postAdopt = opts?.postAdopt === true
  const [cl8yCandidates, terraport] = await Promise.all([discoverCl8yCandidates(addr), discoverTerraport(addr)])
  const cl8y: Cl8yVenueRow[] = []
  for (const candidate of cl8yCandidates) {
    const row = await enrichCl8yRow(addr, candidate, postAdopt)
    if (row) cl8y.push(row)
  }
  cl8y.sort((a, b) => a.pair.localeCompare(b.pair))
  return {
    cl8y,
    otherDex: terraport.rows,
    terraportIncomplete: terraport.incomplete,
  }
}

/** Factory-verify again immediately before execute. Never register a non-factory addr. */
export async function registerMigrateCl8yPair(input: { wallet: string; token: string; pair: string }): Promise<string> {
  const verified = await verifyFactoryListedPair(input.pair)
  if (!verified || !pairContainsToken(verified.asset_infos, input.token)) {
    throw new Error('That pool is not a CL8Y market.')
  }
  return registerListedPair(input.wallet, input.token, verified.contract_addr)
}
