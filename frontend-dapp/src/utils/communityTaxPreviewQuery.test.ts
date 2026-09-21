import { describe, expect, it, vi } from 'vitest'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    ROUTER_CONTRACT_ADDRESS: 'terra1router00000000000000000000000000001',
  }
})

import { tokenAssetInfo, type PairInfo } from '@/types'
import type { SwapOperation } from '@/services/terraclassic/router'
import {
  encodePairSwapTaxPreviewSendMsg,
  encodeRouterHopSwapTaxPreviewSendMsg,
  resolveCommunityTaxPreviewQuery,
  resolveRouterSellHopPairAddress,
} from './communityTaxPreviewQuery'

const WALLET = 'terra1wallet000000000000000000000000000001'
const ROUTER = 'terra1router00000000000000000000000000001'
const PAY = 'terra1pay00000000000000000000000000000001'
const OTHER = 'terra1other000000000000000000000000000001'
const PAIR = 'terra1pair00000000000000000000000000000001'

function pairInfo(): PairInfo {
  return {
    contract_addr: PAIR,
    asset_infos: [tokenAssetInfo(PAY), tokenAssetInfo(OTHER)],
    liquidity_token: 'terra1lp',
    pair_type: {},
  } as PairInfo
}

describe('communityTaxPreviewQuery (#1285)', () => {
  it('pair-direct query includes Swap send_msg to pair', () => {
    const q = resolveCommunityTaxPreviewQuery({
      wallet: WALLET,
      payToken: PAY,
      usesRouter: false,
      directPairAddr: PAIR,
      routeOps: null,
      pairs: [pairInfo()],
      maxSpread: '0.05',
    })
    expect(q).toEqual({
      from: WALLET,
      to: PAIR,
      sendMsg: encodePairSwapTaxPreviewSendMsg({ maxSpread: '0.05' }),
    })
    const inner = JSON.parse(atob(q!.sendMsg)) as { swap?: { max_spread?: string } }
    expect(inner.swap?.max_spread).toBe('0.05')
  })

  it('router query uses router→hop pair with trader on Swap hook', () => {
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: tokenAssetInfo(PAY),
          ask_asset_info: tokenAssetInfo(OTHER),
        },
      },
    ]
    expect(resolveRouterSellHopPairAddress({ payToken: PAY, routeOps: ops, pairs: [pairInfo()] })).toBe(PAIR)
    const q = resolveCommunityTaxPreviewQuery({
      wallet: WALLET,
      payToken: PAY,
      usesRouter: true,
      directPairAddr: PAIR,
      routeOps: ops,
      pairs: [pairInfo()],
    })
    expect(q?.from).toBe(ROUTER)
    expect(q?.to).toBe(PAIR)
    expect(q?.sendMsg).toBe(encodeRouterHopSwapTaxPreviewSendMsg(WALLET))
    const inner = JSON.parse(atob(q!.sendMsg)) as { swap?: { trader?: string } }
    expect(inner.swap?.trader).toBe(WALLET)
  })

  it('router without resolvable hop pair returns null', () => {
    expect(
      resolveCommunityTaxPreviewQuery({
        wallet: WALLET,
        payToken: PAY,
        usesRouter: true,
        directPairAddr: PAIR,
        routeOps: [],
        pairs: [pairInfo()],
      })
    ).toBeNull()
  })
})
