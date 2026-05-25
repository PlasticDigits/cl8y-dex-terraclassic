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

/** Whether `addr` is listed on the factory pair menu (after LCD pair list has resolved). */
export function isKnownFactoryTradePair(
  addr: string | undefined,
  pairs: readonly { contract_addr: string }[]
): boolean {
  return !!addr && isValidTerraAddress(addr) && pairs.some((p) => p.contract_addr === addr)
}

/**
 * Valid-format `terra1…` deep link that is not in the factory pair list.
 * Requires the factory query to have finished successfully so we do not flash a false positive while loading.
 */
export function getUnknownTradePairRouteParam(
  routePair: string | undefined,
  pairs: readonly { contract_addr: string }[],
  factoryPairsResolved: boolean
): string | null {
  const raw = routePair?.trim()
  if (!factoryPairsResolved || !raw || !isValidTerraAddress(raw)) return null
  if (isKnownFactoryTradePair(raw, pairs)) return null
  return raw
}
