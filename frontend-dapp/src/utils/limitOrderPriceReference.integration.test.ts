import { describe, expect, it } from 'vitest'
import { getPool } from '@/services/terraclassic/pair'
import {
  poolReservesToToken1PerToken0Human,
  resolveLimitOrderPriceRef,
  resolvePairDecimalsForLimitPriceRefFromChain,
} from '@/utils/limitOrderPriceReference'
import type { PairInfo } from '@/types'

const PAIR_ADDR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'

const pairInfo: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra19ehn7w9qxjhulu766skgequq8qjtpts6gtwekjgkg4t4ezuyhlfqr5ghcp',
  asset_infos: [
    { token: { contract_addr: 'terra1t7kqn7qlnnh0up2kf2vgkzraa2g52yzgakae2frd9r5w5qmqlr3sm3anq5' } },
    { token: { contract_addr: 'terra14n45jftyuhdxvl4t7lve5jsmzx0n92wnph6m6h73m8emsq9p6qqs6a3lmt' } },
  ],
}

describe('limit order pool ref integration (GitLab #166)', () => {
  it('resolves chain decimals and pool spot for local EMBER/CORAL', async () => {
    const dec = await resolvePairDecimalsForLimitPriceRefFromChain(pairInfo)
    expect(dec).toEqual({ d0: 6, d1: 6 })

    const pool = await getPool(PAIR_ADDR)
    const spot = poolReservesToToken1PerToken0Human(pool, dec!.d0, dec!.d1)
    expect(spot).not.toBeNull()
    expect(spot!).toBeGreaterThan(0)

    const resolved = resolveLimitOrderPriceRef({
      latestTrade: null,
      indexerPair: null,
      pool,
      pairInfo,
      decimalsOverride: dec,
    })
    expect(resolved.refSource).toBe('pool')
    expect(resolved.refToken1PerToken0).toBeCloseTo(spot!, 8)
  })
})
