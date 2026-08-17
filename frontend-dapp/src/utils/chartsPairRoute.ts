import { isValidTerraAddress } from '@/utils/constants'

/**
 * Charts pair deep-link helpers (GitLab #547).
 * Same Terra bech32 rules as Trade (`tradePairRoute.ts`). Invalid / `javascript:` /
 * HTML segments must never become a `Link` href.
 */

export function isChartsPairRouteParam(addr: string | undefined): addr is string {
  return !!addr && isValidTerraAddress(addr)
}

/** Raw route segment when present but not a valid Terra pair address. */
export function getInvalidChartsPairRouteParam(routePair: string | undefined): string | null {
  const raw = routePair?.trim()
  if (!raw || isValidTerraAddress(raw)) return null
  return raw
}

/**
 * Same-origin Charts path for a pair contract. Returns null when `addr` is not
 * a Terra bech32 address so callers omit the `<Link>` (A1 / T541-2).
 */
export function chartsPairHref(addr: string | undefined): string | null {
  const raw = addr?.trim()
  if (!raw || !isValidTerraAddress(raw)) return null
  return `/charts/${raw}`
}
