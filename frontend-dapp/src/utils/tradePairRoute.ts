import { isValidTerraAddress } from '@/utils/constants'

/** Whether a `/trade/:pairAddr` route segment is a pair contract address. */
export function isTradePairRouteParam(addr: string | undefined): addr is string {
  return !!addr && isValidTerraAddress(addr)
}

/** Raw route segment when present but not a valid Terra pair address. */
export function getInvalidTradePairRouteParam(routePair: string | undefined): string | null {
  const raw = routePair?.trim()
  if (!raw || isValidTerraAddress(raw)) return null
  return raw
}
