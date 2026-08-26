import { describe, it, expect, vi } from 'vitest'

const { CLUNC, CUSTC, UST1 } = vi.hoisted(() => ({
  CLUNC: 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg',
  CUSTC: 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch',
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    WRAPPED_NATIVE_PAIRS: {
      [CLUNC]: 'uluna',
      [CUSTC]: 'uusd',
    },
  }
})

import { provideWrapDefaultOn } from '../poolProvideWrapDefault'

describe('provideWrapDefaultOn (GitLab #661)', () => {
  it('defaults wrap on only for legs with a native equivalent', () => {
    expect(provideWrapDefaultOn(CLUNC)).toBe(true)
    expect(provideWrapDefaultOn(CUSTC)).toBe(true)
    expect(provideWrapDefaultOn(UST1)).toBe(false)
    expect(provideWrapDefaultOn('uluna')).toBe(false)
    expect(provideWrapDefaultOn('tokenA')).toBe(false)
    expect(provideWrapDefaultOn('')).toBe(false)
  })
})
