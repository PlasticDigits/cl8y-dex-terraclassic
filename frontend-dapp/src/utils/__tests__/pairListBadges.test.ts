import { describe, it, expect } from 'vitest'
import { getPairListBadges, FACTORY_PAIRS_MAX_FOR_POOL_LIST } from '../pairListBadges'

describe('getPairListBadges', () => {
  it('marks membership in factory set as router graph', () => {
    const factoryPairAddresses = new Set(['terraPair1', 'terraPair2'])
    expect(getPairListBadges({ pairAddress: 'terraPair1', factoryPairAddresses }).isInFactoryRouterGraph).toBe(true)
  })

  it('marks non-membership for indexer-only rows', () => {
    const factoryPairAddresses = new Set(['terraPair1'])
    expect(getPairListBadges({ pairAddress: 'notListed', factoryPairAddresses }).isInFactoryRouterGraph).toBe(false)
  })
})

describe('FACTORY_PAIRS_MAX_FOR_POOL_LIST', () => {
  it('is a large cap for factory pagination', () => {
    expect(FACTORY_PAIRS_MAX_FOR_POOL_LIST).toBeGreaterThanOrEqual(1000)
  })
})
