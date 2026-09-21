import { describe, expect, it } from 'vitest'
import { isNativeDenom } from '@/types'
import { defaultNativeNeedsWrapInput, defaultNativeWrapHopCount, isNativeUlunaDenom } from '@/utils/nativeWrapSwapHints'
import { estimateSwapNetworkFee } from '@/services/terraclassic/swapNetworkFee'
import { WRAP_GAS_LIMIT, WRAP_ROUTER_COMBO_OVERHEAD_GAS } from '@/utils/constants'
import { gasLimitForRouterExecuteSwapOperations } from '@/services/terraclassic/terraGas'

describe('nativeWrapSwapHints (#587 / #1264)', () => {
  it('only uluna is the fee-paying native', () => {
    expect(isNativeUlunaDenom('uluna')).toBe(true)
    expect(isNativeUlunaDenom('uusd')).toBe(false)
    expect(isNativeDenom('uusd')).toBe(true)
    expect(isNativeDenom('uluna')).toBe(true)
  })

  it('USTC wrap+swap defaults to 2 hops until the route is known', () => {
    expect(
      defaultNativeWrapHopCount({
        payIsNativeDenom: isNativeDenom('uusd'),
        isDirectWrapOrUnwrap: false,
      })
    ).toBe(2)
    expect(
      defaultNativeNeedsWrapInput({
        payIsNativeDenom: isNativeDenom('uusd'),
        isDirectWrapOrUnwrap: false,
      })
    ).toBe(true)
  })

  it('direct wrap stays 1 hop (no combo overhead)', () => {
    expect(
      defaultNativeWrapHopCount({
        payIsNativeDenom: true,
        isDirectWrapOrUnwrap: true,
      })
    ).toBe(1)
  })

  it('known route length wins over the hub default', () => {
    expect(
      defaultNativeWrapHopCount({
        operationsLength: 3,
        payIsNativeDenom: true,
        isDirectWrapOrUnwrap: false,
      })
    ).toBe(3)
  })

  it('USTC wrap+2hop hint matches the 2.71M #587 envelope (G1264-1 / G1264-5)', () => {
    const hopCount = defaultNativeWrapHopCount({
      payIsNativeDenom: isNativeDenom('uusd'),
      isDirectWrapOrUnwrap: false,
    })
    const needsWrap = defaultNativeNeedsWrapInput({
      payIsNativeDenom: isNativeDenom('uusd'),
      isDirectWrapOrUnwrap: false,
    })
    const est = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: needsWrap,
      hopCount,
    })
    expect(est.gasLimit).toBe(
      WRAP_GAS_LIMIT + gasLimitForRouterExecuteSwapOperations(2) + WRAP_ROUTER_COMBO_OVERHEAD_GAS
    )
    expect(est.gasLimit).toBe(2_710_000)
  })
})
