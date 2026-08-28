import { isValidTerraAddress } from '@/utils/constants'
import type { PairDisplayLeg } from '@/utils/tradePairDisplayOrientation'
import { chartsPriceTokenForInverted, matchChartsPriceLeg } from '@/utils/tradePairDisplayOrientation'

/**
 * Charts pair deep-link helpers (GitLab #547 / #680).
 * Same Terra bech32 rules as Trade (`tradePairRoute.ts`). Invalid / `javascript:` /
 * HTML segments must never become a `Link` href.
 *
 * `?price=` names the priced token (display base). Repeated `price` keys: **last** wins.
 * Unknown / hostile values are ignored by the page (product default) — never echoed.
 */

export const CHARTS_PRICE_QUERY_KEY = 'price'
export const CHARTS_PRICE_PARAM_MAX_LEN = 80

export function isChartsPairRouteParam(addr: string | undefined): addr is string {
  return !!addr && isValidTerraAddress(addr)
}

/** Raw route segment when present but not a valid Terra pair address. */
export function getInvalidChartsPairRouteParam(routePair: string | undefined): string | null {
  const raw = routePair?.trim()
  if (!raw || isValidTerraAddress(raw)) return null
  return raw
}

/** Safe token for `?price=` hrefs: pair symbol or a leg contract. Never raw user HTML. */
export function isSafeChartsPriceToken(raw: string | null | undefined): raw is string {
  if (raw == null) return false
  const t = raw.trim()
  if (!t || t.length > CHARTS_PRICE_PARAM_MAX_LEN) return false
  if (isValidTerraAddress(t)) return true
  return /^[A-Za-z][A-Za-z0-9]{0,31}$/.test(t)
}

export type ChartsPairHrefOpts = {
  /** Allowlisted priced-token symbol or leg contract. Hostile values are dropped. */
  price?: string | null
}

/**
 * Same-origin Charts path for a pair contract. Returns null when `addr` is not
 * a Terra bech32 address so callers omit the `<Link>` (A1 / T541-2).
 * Optional `price` is appended only after {@link isSafeChartsPriceToken}.
 */
export function chartsPairHref(addr: string | undefined, opts?: ChartsPairHrefOpts): string | null {
  const raw = addr?.trim()
  if (!raw || !isValidTerraAddress(raw)) return null
  const price = opts?.price?.trim()
  if (price && isSafeChartsPriceToken(price)) {
    return `/charts/${raw}?${CHARTS_PRICE_QUERY_KEY}=${encodeURIComponent(price)}`
  }
  return `/charts/${raw}`
}

function looksHostileChartsPrice(raw: string): boolean {
  const t = raw.trim()
  if (!t) return true
  if (t.length > CHARTS_PRICE_PARAM_MAX_LEN) return true
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
  return false
}

/**
 * Read `price` from a query string or `URLSearchParams`.
 * Repeated keys: last non-empty value. Hostile / overlong → `null` (ignore).
 */
export function parseChartsPriceQuery(search: string | URLSearchParams | null | undefined): string | null {
  if (search == null) return null
  const params =
    typeof search === 'string' ? new URLSearchParams(search.startsWith('?') ? search.slice(1) : search) : search
  const all = params.getAll(CHARTS_PRICE_QUERY_KEY)
  if (all.length === 0) return null
  const last = all[all.length - 1]
  if (last == null) return null
  const trimmed = last.trim()
  if (!trimmed || looksHostileChartsPrice(trimmed)) return null
  return trimmed
}

export type ChartsPriceMatch = 'asset0' | 'asset1'

/**
 * Map a parsed `price` value onto a pair leg. Aliases `USTC` / `LUNC` apply only
 * when that wrap leg is on the pair. Pair contract or a third token → `null`.
 */
export function matchChartsPriceParam(
  raw: string | null | undefined,
  asset0: PairDisplayLeg | null | undefined,
  asset1: PairDisplayLeg | null | undefined,
  pairContract?: string | null
): ChartsPriceMatch | null {
  if (raw == null) return null
  const trimmed = raw.trim()
  if (!trimmed || looksHostileChartsPrice(trimmed)) return null
  const pair = (pairContract ?? '').trim().toLowerCase()
  if (pair && trimmed.toLowerCase() === pair) return null
  return matchChartsPriceLeg(trimmed, asset0, asset1)
}

export function chartsHrefForOrientation(
  pairAddr: string,
  inverted: boolean,
  asset0: PairDisplayLeg | null | undefined,
  asset1: PairDisplayLeg | null | undefined,
  token0Symbol: string,
  token1Symbol: string
): string | null {
  const price = chartsPriceTokenForInverted(inverted, asset0, asset1, token0Symbol, token1Symbol)
  return chartsPairHref(pairAddr, { price })
}
