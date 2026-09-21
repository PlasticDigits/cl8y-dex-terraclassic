import type { IndexerAssetBrief, IndexerPair, IndexerToken } from '@/types'
import { isPairLegDecimals } from '@/utils/formatAmount'
import { lookupByTokenId, USDT_CW20_ADDRESS, USTR_CW20_ADDRESS } from '@/utils/tokenRegistry'

/** Where Swap / Trade market execute amounts got their 0…18 scale (Forgejo #1255). */
export type SwapAssetDecimalsSource = 'registry' | 'lcd' | 'indexer'

/**
 * Parse CW20 `token_info.decimals` for amount math.
 *
 * Accepts JSON numbers and decimal digit strings in `0…18`. Rejects scientific
 * notation (`"18e0"`), floats, negatives, and hostile `255` / `1e9` so callers
 * never `10 ** n` explode.
 */
export function parseTokenInfoDecimals(raw: unknown): number | null {
  if (typeof raw === 'number') {
    if (!Number.isInteger(raw) || raw < 0 || raw > 18) return null
    return raw
  }
  if (typeof raw === 'string') {
    const trimmed = raw.trim()
    if (!/^[0-9]{1,2}$/.test(trimmed)) return null
    const n = Number(trimmed)
    if (!Number.isInteger(n) || n < 0 || n > 18) return null
    return n
  }
  return null
}

/**
 * Registry / columbus-5 USTR+USDT pin for Swap amounts. Unknown CW20 is `null`
 * (never `?? 6`). Natives `uluna` / `uusd` stay 6 via the registry row.
 */
export function registrySwapDecimals(tokenId: string | undefined | null): number | null {
  if (!tokenId?.trim()) return null
  const id = tokenId.trim().toLowerCase()
  if (id === USTR_CW20_ADDRESS.toLowerCase() || id === USDT_CW20_ADDRESS.toLowerCase()) return 18
  const row = lookupByTokenId(tokenId)
  return isPairLegDecimals(row?.decimals) ? row.decimals : null
}

/** Pair-leg decimals keyed by contract / denom identity, never ticker (#1255). */
export function indexerPairLegDecimals(
  pair: Pick<IndexerPair, 'asset_0' | 'asset_1'> | null | undefined,
  tokenId: string | undefined | null
): number | null {
  if (!pair || !tokenId?.trim()) return null
  const id = tokenId.trim().toLowerCase()
  for (const leg of [pair.asset_0, pair.asset_1] as IndexerAssetBrief[]) {
    const addr = leg.contract_addr?.trim().toLowerCase()
    const denom = leg.denom?.trim().toLowerCase()
    if (addr === id || denom === id) {
      return isPairLegDecimals(leg.decimals) ? leg.decimals : null
    }
  }
  return null
}

/** `GET /tokens` decimals for the same asset identity. */
export function indexerTokenDecimals(token: Pick<IndexerToken, 'decimals'> | null | undefined): number | null {
  return isPairLegDecimals(token?.decimals) ? token.decimals : null
}

export type ResolveSwapAssetDecimalsInput = {
  registryDecimals: number | null
  indexerDecimals: number | null
  onChainDecimals: number | null
  /** LCD `token_info` finished (success or transport error), not in-flight. */
  lcdSettled: boolean
  /** LCD returned a payload whose `decimals` is out of `0…18`. */
  lcdHostile: boolean
}

/**
 * Swap execute-amount precedence (Forgejo #1255):
 *
 * 1. Registry / USTR+USDT pin (listed product ids).
 * 2. Valid LCD `token_info.decimals` — wins over a stale indexer 6 when both exist.
 * 3. Indexer pair-leg / `GET /tokens` while LCD is in flight or transport-failed.
 * 4. Hostile LCD → unresolved (no indexer fallback). Missing / pending → `null`.
 */
export function resolveSwapAssetDecimals(input: ResolveSwapAssetDecimalsInput): {
  decimals: number | null
  source: SwapAssetDecimalsSource | null
} {
  const registry = isPairLegDecimals(input.registryDecimals) ? input.registryDecimals : null
  if (registry != null) return { decimals: registry, source: 'registry' }

  if (input.lcdHostile) return { decimals: null, source: null }

  const lcd = isPairLegDecimals(input.onChainDecimals) ? input.onChainDecimals : null
  if (lcd != null) return { decimals: lcd, source: 'lcd' }

  const indexer = isPairLegDecimals(input.indexerDecimals) ? input.indexerDecimals : null
  if (indexer != null && (!input.lcdSettled || lcd == null)) {
    return { decimals: indexer, source: 'indexer' }
  }

  return { decimals: null, source: null }
}
