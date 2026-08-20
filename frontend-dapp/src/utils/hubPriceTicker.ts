/** Allowlisted DEX hub USD tickers (GitLab #556 / #570). Not CEX `OracleTicker`. */

export const HUB_PRICE_TICKERS = ['custc', 'lunc', 'ust1', 'ustr'] as const

export type HubPriceTicker = (typeof HUB_PRICE_TICKERS)[number]

export const HUB_PRICE_TICKER_LABEL: Record<HubPriceTicker, string> = {
  custc: 'cUSTC',
  lunc: 'LUNC',
  ust1: 'UST1',
  ustr: 'USTR',
}

/** Oracle-anchored wraps get a token AddressRow (not a fake source_pair). */
export const HUB_ORACLE_WRAP_TICKERS = ['custc', 'lunc'] as const satisfies readonly HubPriceTicker[]

export type HubOracleWrapTicker = (typeof HUB_ORACLE_WRAP_TICKERS)[number]

export function isHubOracleWrapTicker(ticker: HubPriceTicker): ticker is HubOracleWrapTicker {
  return ticker === 'custc' || ticker === 'lunc'
}

const ALLOWLIST = new Set<string>(HUB_PRICE_TICKERS)

/**
 * Parse a hub ticker path. Unknown / HTML / `javascript:` / `../` → `null`.
 * Never interpolate raw input into fetch URLs. No `clunc` alias.
 */
export function parseHubPriceTicker(raw: unknown): HubPriceTicker | null {
  if (typeof raw !== 'string') return null
  const trimmed = raw.trim().toLowerCase()
  if (ALLOWLIST.has(trimmed)) return trimmed as HubPriceTicker
  return null
}
