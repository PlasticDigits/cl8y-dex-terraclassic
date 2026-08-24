import { describe, expect, it } from 'vitest'
import {
  pairDirectSwapHook,
  pairDirectSwapSetsTrader,
  routerExecuteSwapOperations,
  routerHopSwapPreviewHook,
} from './taxHooks.js'

describe('pair-direct Swap must not spoof trader (GitLab #621)', () => {
  it('leaves trader unset', () => {
    const hook = pairDirectSwapHook()
    expect(hook.swap.trader).toBeUndefined()
    expect(pairDirectSwapSetsTrader(hook)).toBe(false)
  })

  it('detects a spoofed trader (bots must not do this)', () => {
    expect(pairDirectSwapSetsTrader({ swap: { trader: 'terra1victim' } })).toBe(true)
  })
})

describe('router execute_swap_operations', () => {
  it('does not put trader on the user Send hook (router stamps Swap.trader)', () => {
    const inner = routerExecuteSwapOperations([
      { terra_swap: { offer_asset_info: { token: { contract_addr: 'terra1a' } } } },
    ])
    const ops = inner.execute_swap_operations as { trader?: unknown }
    expect(ops.trader).toBeUndefined()
  })

  it('preview hop hook sets authenticated trader for TaxPreview only', () => {
    const hop = routerHopSwapPreviewHook('terra1bot')
    expect((hop.swap as { trader: string }).trader).toBe('terra1bot')
  })
})
