/**
 * Create Pair `/create?a=&b=` prefill (GitLab #713). Catalog + checksum paste only —
 * never Swap's factory graph (C542-8). Hostile / native / bad checksum ignored per side.
 */
import { listedCreatePairAddress, sameCreatePairAddress } from '@/utils/createPairTokenCatalog'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { SWAP_QUERY_VALUE_MAX_LEN } from '@/utils/swapQueryParams'

export const CREATE_PAIR_A_KEYS = ['a', 'tokenA', 'token_a'] as const
export const CREATE_PAIR_B_KEYS = ['b', 'tokenB', 'token_b'] as const

const NATIVE_DENOMS = new Set(['uluna', 'uusd'])

export type CreatePairQueryParse = {
  tokenA: string | null
  tokenB: string | null
}

function looksHostile(raw: string): boolean {
  const t = raw.trim()
  if (!t || t.length > SWAP_QUERY_VALUE_MAX_LEN) return true
  if (/[<>"'`\\/?#\s]/.test(t)) return true
  const lower = t.toLowerCase()
  if (
    lower.startsWith('javascript:') ||
    lower.startsWith('data:') ||
    lower.startsWith('http:') ||
    lower.startsWith('https:') ||
    lower.startsWith('//')
  ) {
    return true
  }
  if (lower.startsWith('0x')) return true
  if (lower.startsWith('ibc/')) return true
  if (lower.startsWith('factory/')) return true
  return false
}

function lastNonEmptyForKey(params: URLSearchParams, key: string): string | null {
  const want = key.toLowerCase()
  let last: string | null = null
  for (const [k, v] of params.entries()) {
    if (k.toLowerCase() !== want) continue
    const trimmed = v.trim()
    if (trimmed) last = trimmed
  }
  return last
}

function firstFamilyValue(params: URLSearchParams, keys: readonly string[]): string | null {
  for (const key of keys) {
    const v = lastNonEmptyForKey(params, key)
    if (v) return v
  }
  return null
}

function toSearchParams(search: string | URLSearchParams | null | undefined): URLSearchParams {
  if (search == null) return new URLSearchParams()
  if (typeof search === 'string') {
    return new URLSearchParams(search.startsWith('?') ? search.slice(1) : search)
  }
  return search
}

/**
 * Resolve one Create Pair query value: catalog hit or checksummed `terra1` custom paste.
 * Natives, hostile strings, and bad checksums → `null` (never echo).
 */
export function resolveCreatePairQueryValue(raw: string | null | undefined, catalog: readonly string[]): string | null {
  if (raw == null) return null
  const trimmed = raw.trim()
  if (!trimmed || looksHostile(trimmed)) return null
  const lower = trimmed.toLowerCase()
  if (NATIVE_DENOMS.has(lower) || lower === 'lunc' || lower === 'ustc' || lower === 'ust') return null
  const listed = listedCreatePairAddress(catalog, trimmed)
  if (listed) return listed
  if (lower.startsWith('terra1') && isValidTerraBech32Address(trimmed)) return trimmed
  return null
}

/**
 * Parse `/create` search. Same token both sides → drop B. One-sided query is OK.
 */
export function parseCreatePairQuery(
  search: string | URLSearchParams | null | undefined,
  catalog: readonly string[]
): CreatePairQueryParse {
  const params = toSearchParams(search)
  const a = resolveCreatePairQueryValue(firstFamilyValue(params, CREATE_PAIR_A_KEYS), catalog)
  let b = resolveCreatePairQueryValue(firstFamilyValue(params, CREATE_PAIR_B_KEYS), catalog)
  if (a && b && sameCreatePairAddress(a, b)) {
    b = null
  }
  return { tokenA: a, tokenB: b }
}

/** Canonical `?a=&b=` with bech32 only. Omits empty sides. */
export function canonicalCreatePairSearch(parsed: CreatePairQueryParse): URLSearchParams {
  const q = new URLSearchParams()
  if (parsed.tokenA) q.set('a', parsed.tokenA)
  if (parsed.tokenB) q.set('b', parsed.tokenB)
  return q
}
