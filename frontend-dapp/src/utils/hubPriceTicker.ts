/** Allowlisted DEX hub USD tickers (GitLab #556). Not CEX `OracleTicker`. */

export const HUB_PRICE_TICKERS = ['custc', 'ust1', 'ustr'] as const

export type HubPriceTicker = (typeof HUB_PRICE_TICKERS)[number]

export const HUB_PRICE_TICKER_LABEL: Record<HubPriceTicker, string> = {
  custc: 'cUSTC',
  ust1: 'UST1',
  ustr: 'USTR',
}

const ALLOWLIST = new Set<string>(HUB_PRICE_TICKERS)

/**
 * Parse a hub ticker path. Unknown / HTML / `javascript:` / `../` → `null`.
 * Never interpolate raw input into fetch URLs.
 */
export function parseHubPriceTicker(raw: unknown): HubPriceTicker | null {
  if (typeof raw !== 'string') return null
  const trimmed = raw.trim().toLowerCase()
  if (ALLOWLIST.has(trimmed)) return trimmed as HubPriceTicker
  return null
}
