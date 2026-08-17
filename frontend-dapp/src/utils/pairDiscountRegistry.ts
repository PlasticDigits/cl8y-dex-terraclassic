/**
 * Pair-scoped CL8Y fee-tier chrome (GitLab #537 / invariant I14).
 *
 * On-chain `lookup_effective_fee_bps_cached` uses the pair's `DISCOUNT_REGISTRY`.
 * When that is `None`, swaps charge full `fee_bps` and maker place is
 * `maker_fee_bps(fee_bps)` — regardless of the wallet's registry `get_discount`.
 * The dApp must not strikethrough or advertise a VITE_FEE_DISCOUNT_ADDRESS
 * discount unless the pair registry is set and matches that contract.
 */

/** cw-storage-plus Item key (`DISCOUNT_REGISTRY`). LCD `/raw/` uses base64 of these bytes. */
export const PAIR_DISCOUNT_REGISTRY_STORAGE_KEY = 'discount_registry'

/** LCD raw-state key; fallback when `GetDiscountRegistry` is missing (1.13.x). */
export function pairDiscountRegistryRawKeyB64(): string {
  return btoa(PAIR_DISCOUNT_REGISTRY_STORAGE_KEY)
}

export function normalizeTerraAddr(value: string | null | undefined): string | null {
  const t = (value ?? '').trim()
  if (!t) return null
  const lower = t.toLowerCase()
  // `terra1` alone is not an address; keep this loose so test fixtures like
  // `terra1feediscount` still match (mainnet addrs are ~44 chars).
  if (!lower.startsWith('terra1') || lower.length <= 6) return null
  return lower
}

/**
 * Decode LCD `/raw/` `data` for `Item<Option<Addr>>` (JSON `null` or `"terra1…"`).
 */
export function decodeCwStoragePlusOptionalAddr(rawB64: string | null | undefined): string | null {
  if (rawB64 == null || rawB64 === '') return null
  let decoded: string
  try {
    decoded = atob(rawB64)
  } catch {
    return null
  }
  const trimmed = decoded.trim()
  if (!trimmed || trimmed === 'null') return null
  try {
    const parsed: unknown = JSON.parse(trimmed)
    if (parsed == null) return null
    if (typeof parsed === 'string') return normalizeTerraAddr(parsed)
    return null
  } catch {
    return normalizeTerraAddr(trimmed)
  }
}

export function parseGetDiscountRegistryResponse(resp: unknown): string | null {
  if (resp == null || typeof resp !== 'object') return null
  const rec = resp as Record<string, unknown>
  const value = rec.discount_registry ?? rec.registry
  if (value == null) return null
  if (typeof value === 'string') return normalizeTerraAddr(value)
  return null
}

/** True only when the pair is wired to the same fee-discount contract the dApp queries. */
export function pairFeeDiscountApplies(
  pairRegistry: string | null | undefined,
  configuredRegistry: string | null | undefined
): boolean {
  const pair = normalizeTerraAddr(pairRegistry)
  const configured = normalizeTerraAddr(configuredRegistry)
  if (!pair || !configured) return false
  return pair.toLowerCase() === configured.toLowerCase()
}

/**
 * Discount bps the UI may advertise (strikethrough / maker place). Probe unknown or
 * mismatch → 0 (fail-closed; do not invent a client-side discount).
 */
export function advertisedDiscountBps(
  traderDiscountBps: number | null | undefined,
  pairRegistry: string | null | undefined,
  configuredRegistry: string | null | undefined
): number {
  if (!pairFeeDiscountApplies(pairRegistry, configuredRegistry)) return 0
  const n = Math.floor(Number(traderDiscountBps ?? 0))
  if (!Number.isFinite(n) || n <= 0) return 0
  return Math.min(n, 10000)
}
