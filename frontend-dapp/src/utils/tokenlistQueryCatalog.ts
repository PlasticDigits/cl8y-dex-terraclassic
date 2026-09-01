/**
 * Bundled tokenlist → Swap query encode/decode (GitLab #715).
 *
 * Create Pair catalog reuses the same Vite overlays (**C542-5**). Do not fetch
 * `tokenlist.json` at runtime. Do not use LCD `token_info.symbol` as a query key (**X1**).
 */
import publishedTokenlist from '../../../tokenlist/tokenlist.json'
import {
  CL8Y_TOKEN_ADDRESS,
  LUNC_C_TOKEN_ADDRESS,
  UST1_TOKEN_ADDRESS,
  USTC_C_TOKEN_ADDRESS,
  VFDUSD_TOKEN_ADDRESS,
} from '@/utils/constants'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'

export type TokenlistQueryRow = {
  symbol: string
  name?: string
  address?: string
  denom?: string
  type: string
}

export type TokenlistQueryMaps = {
  /** ASCII-folded symbol → execute id (overlay wins). */
  symbolToId: ReadonlyMap<string, string>
  /** Lowercase execute id → published symbol casing. */
  idToSymbol: ReadonlyMap<string, string>
}

/** ASCII A–Z fold only — Cyrillic lookalikes never match `UST1`. */
export function foldTokenlistSymbol(raw: string): string {
  let out = ''
  for (const ch of raw.trim()) {
    const code = ch.charCodeAt(0)
    out += code >= 65 && code <= 90 ? String.fromCharCode(code + 32) : ch
  }
  return out
}

/** Published tickers are letters + digits (`cLUNC`, `vFDUSD`, `SpaceUSD`, `UST1`). */
export function isAsciiTokenlistSymbol(raw: string): boolean {
  return /^[A-Za-z0-9]+$/.test(raw.trim())
}

/**
 * LocalTerra / Vite address overlays. Empty overlay keeps the published row.
 * Keys match published `symbol` casing (**C542-5**).
 */
export function tokenlistViteOverlays(): Readonly<Record<string, string>> {
  return {
    cLUNC: LUNC_C_TOKEN_ADDRESS,
    cUSTC: USTC_C_TOKEN_ADDRESS,
    UST1: UST1_TOKEN_ADDRESS,
    vFDUSD: VFDUSD_TOKEN_ADDRESS,
    CL8Y: CL8Y_TOKEN_ADDRESS,
  }
}

export function overlayOrPublishedAddress(
  symbol: string,
  published: string,
  overlays: Readonly<Record<string, string>>
): string {
  const overlay = (overlays[symbol] ?? '').trim()
  return overlay || published.trim()
}

function executeIdForRow(row: TokenlistQueryRow, overlays: Readonly<Record<string, string>>): string | null {
  const symbol = row.symbol?.trim() ?? ''
  if (!symbol || !isAsciiTokenlistSymbol(symbol)) return null
  if (row.type === 'native') {
    const denom = (row.denom ?? '').trim().toLowerCase()
    return denom || null
  }
  if (row.type === 'cw20') {
    const address = overlayOrPublishedAddress(symbol, row.address ?? '', overlays)
    if (!address || !isValidTerraBech32Address(address)) return null
    return address
  }
  return null
}

/**
 * Build bidirectional maps from bundled rows + overlays.
 * Duplicate folded symbols keep the first row (CI must reject that list).
 * Overlay and published CW20 addresses both reverse-map to the published symbol.
 */
export function buildTokenlistQueryMaps(
  rows: readonly TokenlistQueryRow[],
  overlays: Readonly<Record<string, string>> = {}
): TokenlistQueryMaps {
  const symbolToId = new Map<string, string>()
  const idToSymbol = new Map<string, string>()

  for (const row of rows) {
    const symbol = row.symbol?.trim() ?? ''
    const executeId = executeIdForRow(row, overlays)
    if (!executeId) continue
    const folded = foldTokenlistSymbol(symbol)
    if (!symbolToId.has(folded)) {
      symbolToId.set(folded, executeId)
    }
    idToSymbol.set(executeId.toLowerCase(), symbol)
    if (row.type === 'cw20') {
      const published = (row.address ?? '').trim()
      if (published && published.toLowerCase() !== executeId.toLowerCase()) {
        idToSymbol.set(published.toLowerCase(), symbol)
      }
    }
  }

  return { symbolToId, idToSymbol }
}

let liveMapsCache: TokenlistQueryMaps | null = null

export function publishedTokenlistQueryRows(): readonly TokenlistQueryRow[] {
  const published = publishedTokenlist as { tokens: readonly TokenlistQueryRow[] }
  return published.tokens
}

export function liveTokenlistQueryMaps(): TokenlistQueryMaps {
  if (!liveMapsCache) {
    liveMapsCache = buildTokenlistQueryMaps(publishedTokenlistQueryRows(), tokenlistViteOverlays())
  }
  return liveMapsCache
}

/** Unique tokenlist symbol → overlay-or-published execute id. Unknown → `null`. */
export function queryTokenToExecuteId(raw: string): string | null {
  const folded = foldTokenlistSymbol(raw)
  if (!folded) return null
  return liveTokenlistQueryMaps().symbolToId.get(folded) ?? null
}

/**
 * Execute id → published tokenlist symbol when unique; else the execute id
 * (`uluna` / `uusd` / checksummed `terra1`).
 */
export function executeIdToQueryToken(id: string): string {
  const trimmed = id.trim()
  if (!trimmed) return trimmed
  return liveTokenlistQueryMaps().idToSymbol.get(trimmed.toLowerCase()) ?? trimmed
}
