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
): addr is string {
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

/**
 * Valid-format `/trade/:pairAddr` deep link while the factory pair list is still loading.
 * Avoid rendering an empty trade workspace before unknown-pair detection can run.
 */
export function isPendingTradePairRouteResolution(
  routePair: string | undefined,
  invalidRoutePair: string | null,
  factoryPairsResolved: boolean
): boolean {
  const raw = routePair?.trim()
  if (!raw || invalidRoutePair || !isValidTerraAddress(raw)) return false
  return !factoryPairsResolved
}

/**
 * Whether `/trade` (no deep link to honor) should navigate to the first factory pair.
 * Skips when a valid `:pairAddr` segment is present — known-pair sync owns that case
 * (GitLab #357).
 */
export function shouldAutoPickDefaultTradePair(opts: {
  routePair: string | undefined
  invalidRoutePair: string | null
  unknownRoutePair: string | null
  pendingDeepLinkPair: boolean
}): boolean {
  if (opts.invalidRoutePair || opts.unknownRoutePair || opts.pendingDeepLinkPair) return false
  if (isTradePairRouteParam(opts.routePair?.trim())) return false
  return true
}

/** Whether book/chart/ticket workspace should mount (pair selected and no link-error notices). */
export function shouldShowTradeWorkspace(opts: {
  pairRouteReady: boolean
  invalidLinkNotice: string | null
  unknownPairNotice: string | null
  pendingDeepLinkPair: boolean
}): boolean {
  return opts.pairRouteReady && !opts.invalidLinkNotice && !opts.unknownPairNotice && !opts.pendingDeepLinkPair
}
