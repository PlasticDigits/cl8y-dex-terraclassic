/**
 * Create Pair listed-CW20 catalog (GitLab #542).
 *
 * Convenience list only — factory code-ID whitelist + on-chain TokenInfo stay
 * authoritative. Do not import the Swap factory pair graph. Do not load a
 * remote token list at runtime. Do not use tokenlist `decimals` for amounts
 * (Create Pair sends no amounts).
 */
import publishedTokenlist from '../../../tokenlist/tokenlist.json'
import { SOFT_LAUNCH_MINTABLE_TOKENS } from '@/utils/constants'
import { compareTokenCatalog, retailExposeTestTokens } from '@/utils/pairCatalogRank'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { overlayOrPublishedAddress, tokenlistViteOverlays } from '@/utils/tokenlistQueryCatalog'

export type CreatePairCw20Option = {
  address: string
  symbol: string
  name: string
}

export type CreatePairTokenlistRow = {
  symbol: string
  name: string
  address?: string
  denom?: string
  type: string
}

export type CreatePairCatalogInput = {
  tokenlistTokens: readonly CreatePairTokenlistRow[]
  /** Symbol → env overlay (empty = use published tokenlist address). */
  overlays: Readonly<Record<string, string>>
  gems: readonly { symbol: string; address: string; name?: string }[]
}

const NATIVE_DENOMS = new Set(['uluna', 'uusd'])

function isCatalogCw20Address(addr: string): boolean {
  const trimmed = addr.trim()
  return trimmed.length > 0 && isValidTerraBech32Address(trimmed)
}

/**
 * Build the Create Pair picker universe from a bundled tokenlist + env overlays + optional gems.
 * Drops natives, empty/invalid bech32, and duplicate addresses (lowercase).
 */
export function buildCreatePairCw20Options(input: CreatePairCatalogInput): CreatePairCw20Option[] {
  const rows: CreatePairCw20Option[] = []

  for (const token of input.tokenlistTokens) {
    if (token.type !== 'cw20') continue
    const denom = token.denom?.trim().toLowerCase() ?? ''
    if (denom && NATIVE_DENOMS.has(denom)) continue
    const published = token.address?.trim() ?? ''
    const address = overlayOrPublishedAddress(token.symbol, published, input.overlays)
    if (!isCatalogCw20Address(address)) continue
    rows.push({
      address,
      symbol: token.symbol,
      name: token.name,
    })
  }

  for (const gem of input.gems) {
    const address = gem.address.trim()
    if (!isCatalogCw20Address(address)) continue
    rows.push({
      address,
      symbol: gem.symbol,
      name: gem.name?.trim() || gem.symbol,
    })
  }

  const seen = new Set<string>()
  const deduped: CreatePairCw20Option[] = []
  for (const row of rows) {
    const key = row.address.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    deduped.push(row)
  }

  return deduped.sort((a, b) => compareTokenCatalog(a.address, b.address))
}

/** Live catalog: bundled `tokenlist.json` + Vite env overlays + gems when `retailExposeTestTokens()`. */
export function getCreatePairCw20Options(): CreatePairCw20Option[] {
  const published = publishedTokenlist as { tokens: readonly CreatePairTokenlistRow[] }
  return buildCreatePairCw20Options({
    tokenlistTokens: published.tokens,
    overlays: tokenlistViteOverlays(),
    gems: retailExposeTestTokens()
      ? SOFT_LAUNCH_MINTABLE_TOKENS.map((t) => ({
          symbol: t.symbol,
          address: t.address,
          name: t.symbol,
        }))
      : [],
  })
}

export function getCreatePairCw20Addresses(): string[] {
  return getCreatePairCw20Options().map((row) => row.address)
}

/** Case-insensitive match against the gated catalog (picker `onChange` guard). */
export function listedCreatePairAddress(catalog: readonly string[], candidate: string): string | undefined {
  const lower = candidate.trim().toLowerCase()
  if (!lower) return undefined
  return catalog.find((addr) => addr.toLowerCase() === lower)
}

export function sameCreatePairAddress(a: string, b: string): boolean {
  const left = a.trim().toLowerCase()
  const right = b.trim().toLowerCase()
  return left.length > 0 && left === right
}
