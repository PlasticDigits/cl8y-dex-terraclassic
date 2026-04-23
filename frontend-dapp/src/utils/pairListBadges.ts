/**
 * Pool list: relate indexer rows to on-chain factory registration.
 * Used by PoolPage to avoid per-card chain queries (see docs/frontend.md#liquidity-pools-list-indexer-vs-factory).
 */

/** Max pair contracts to pull from the factory in one pool-list session (paginated LCD queries). */
export const FACTORY_PAIRS_MAX_FOR_POOL_LIST = 10_000

export interface PairListBadgesInput {
  pairAddress: string
  /** `Set` of pair `contract_addr` values from `getAllPairsPaginated` (factory `pairs` query). */
  factoryPairAddresses: Set<string>
}

export interface PairListBadges {
  /**
   * Pair contract appears in the factory’s paginated `pairs` list — the same set Swap uses
   * to build the token graph (`findRoute` / `getAllTokens`).
   */
  isInFactoryRouterGraph: boolean
}

export function getPairListBadges(input: PairListBadgesInput): PairListBadges {
  return {
    isInFactoryRouterGraph: input.factoryPairAddresses.has(input.pairAddress),
  }
}
