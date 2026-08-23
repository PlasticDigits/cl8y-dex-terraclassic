/**
 * Create Token identity + connected-wallet helpers (GitLab #604).
 * Shared by the page and hook builders so paid and free create cannot encode illegal metadata.
 */

export const COMMUNITY_TAX_MIN_DECIMALS = 6
export const COMMUNITY_TAX_MAX_DECIMALS = 18
export const COMMUNITY_TAX_NAME_MIN = 3
export const COMMUNITY_TAX_NAME_MAX = 50
export const COMMUNITY_TAX_SYMBOL_MIN = 3
export const COMMUNITY_TAX_SYMBOL_MAX = 12
export const COMMUNITY_TAX_IDENTITY_CHARSET = /^[A-Za-z0-9]+$/
export const CONNECTED_WALLET_HELPER = 'connected wallet'
export const NOT_CONNECTED_WALLET_HELPER = 'not connected wallet'

export type ParseOk<T> = { ok: true; value: T }
export type ParseErr = { ok: false; error: string }
export type ParseResult<T> = ParseOk<T> | ParseErr

export function parseTokenName(raw: string): ParseResult<string> {
  const name = raw.trim()
  if (!name) return { ok: false, error: 'Name must be 3–50 letters or numbers' }
  if (name.length < COMMUNITY_TAX_NAME_MIN || name.length > COMMUNITY_TAX_NAME_MAX) {
    return { ok: false, error: 'Name must be 3–50 letters or numbers' }
  }
  if (!COMMUNITY_TAX_IDENTITY_CHARSET.test(name)) {
    return { ok: false, error: 'Name may contain only letters and numbers' }
  }
  return { ok: true, value: name }
}

export function parseTokenSymbol(raw: string): ParseResult<string> {
  const symbol = raw.trim()
  if (!symbol) return { ok: false, error: 'Symbol must be 3–12 letters or numbers' }
  if (symbol.length < COMMUNITY_TAX_SYMBOL_MIN || symbol.length > COMMUNITY_TAX_SYMBOL_MAX) {
    return { ok: false, error: 'Symbol must be 3–12 letters or numbers' }
  }
  if (!COMMUNITY_TAX_IDENTITY_CHARSET.test(symbol)) {
    return { ok: false, error: 'Symbol may contain only letters and numbers' }
  }
  return { ok: true, value: symbol.toUpperCase() }
}

export function parseTokenDecimals(raw: string): ParseResult<number> {
  const trimmed = raw.trim()
  if (!trimmed) return { ok: false, error: 'Decimals must be a whole number from 6 to 18' }
  if (!/^\d+$/.test(trimmed)) {
    return { ok: false, error: 'Decimals must be a whole number from 6 to 18' }
  }
  const dec = Number(trimmed)
  if (!Number.isInteger(dec) || dec < COMMUNITY_TAX_MIN_DECIMALS || dec > COMMUNITY_TAX_MAX_DECIMALS) {
    return { ok: false, error: 'Decimals must be a whole number from 6 to 18' }
  }
  return { ok: true, value: dec }
}

/** Bech32 compare: Terra addresses are case-insensitive. */
export function normalizeTerraAddr(addr: string): string {
  return addr.trim().toLowerCase()
}

export function isSameTerraAddr(a: string, b: string | null | undefined): boolean {
  if (!a.trim() || !b?.trim()) return false
  return normalizeTerraAddr(a) === normalizeTerraAddr(b)
}

/** Retail helper copy — exact strings required by #604. */
export function walletOwnershipHelper(value: string, connected: string | null | undefined): string {
  return isSameTerraAddr(value, connected) ? CONNECTED_WALLET_HELPER : NOT_CONNECTED_WALLET_HELPER
}

/** Fill empty treasury/manager with the connected wallet; never clobber a typed value. */
export function autofillConnectedWallet(current: string, connected: string | null | undefined): string {
  if (current.trim()) return current
  return connected?.trim() ?? current
}
