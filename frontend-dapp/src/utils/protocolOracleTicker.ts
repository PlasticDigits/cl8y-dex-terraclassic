/** Allowlisted Protocol / indexer oracle tickers (GitLab #550). */

export const PROTOCOL_ORACLE_TICKERS = ['ustc', 'lunc', 'vfdusd'] as const

export type ProtocolOracleTicker = (typeof PROTOCOL_ORACLE_TICKERS)[number]

export const PROTOCOL_ORACLE_TICKER_LABEL: Record<ProtocolOracleTicker, string> = {
  ustc: 'USTC',
  lunc: 'LUNC',
  vfdusd: 'vFDUSD',
}

const ALLOWLIST = new Set<string>(PROTOCOL_ORACLE_TICKERS)

/**
 * Parse a user/query ticker. Unknown, HTML, `javascript:`, `../`, `fdusd` → `ustc`.
 * Never pass raw query input to fetch URLs.
 */
export function parseProtocolOracleTicker(raw: unknown): ProtocolOracleTicker {
  if (typeof raw !== 'string') return 'ustc'
  const trimmed = raw.trim().toLowerCase()
  if (ALLOWLIST.has(trimmed)) {
    return trimmed as ProtocolOracleTicker
  }
  return 'ustc'
}

export function isProtocolOracleTicker(raw: string): raw is ProtocolOracleTicker {
  return ALLOWLIST.has(raw.trim().toLowerCase())
}
