import { describe, expect, it } from 'vitest'
import { getPool } from '@/services/terraclassic/pair'
import {
  poolReservesToToken1PerToken0Human,
  resolveLimitOrderPriceRef,
  resolvePairDecimalsForLimitPriceRefFromChain,
} from '@/utils/limitOrderPriceReference'
import {
  LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS,
  limitOrderIntegrationPairInfo,
} from '@/test/limitOrderIntegrationConstants'

const hasLimitOrderFixture = Boolean(import.meta.env.VITE_LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS)

describe.skipIf(!hasLimitOrderFixture)('limit order pool ref integration (GitLab #166)', () => {
  it('resolves chain decimals and pool spot for local EMBER/CORAL', async () => {
    const dec = await resolvePairDecimalsForLimitPriceRefFromChain(limitOrderIntegrationPairInfo)
    expect(dec).toEqual({ d0: 6, d1: 6 })

    const pool = await getPool(LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS)
    const spot = poolReservesToToken1PerToken0Human(pool, dec!.d0, dec!.d1)
    expect(spot).not.toBeNull()
    expect(spot!).toBeGreaterThan(0)

    const resolved = resolveLimitOrderPriceRef({
      latestTrade: null,
      indexerPair: null,
      pool,
      pairInfo: limitOrderIntegrationPairInfo,
      decimalsOverride: dec,
    })
    expect(resolved.refSource).toBe('pool')
    expect(resolved.refToken1PerToken0).toBeCloseTo(spot!, 8)
  })
})
