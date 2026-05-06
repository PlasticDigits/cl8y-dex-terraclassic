/**
 * Surfaces indexer failures during the post–limit-place poll (CORS, 5xx, parse).
 * Silent catches made local-only regressions indistinguishable from “indexer down”.
 * @see https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131
 */
export function warnIndexerPlacementPollFailed(err: unknown): void {
  console.warn('[limit-place] indexer poll failed:', err)
}
