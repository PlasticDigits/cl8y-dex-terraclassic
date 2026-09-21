/**
 * Execute-aligned `TaxPreview` query params (#1285 / S1285-1–S1285-3).
 * Pair-direct Sell includes pair `Swap` send_msg; router hops preview router→pair with `trader`.
 */

import { ROUTER_CONTRACT_ADDRESS } from '@/utils/constants'
import type { SwapOperation } from '@/services/terraclassic/router'
import { assetInfoLabel, type PairInfo } from '@/types'

export type CommunityTaxPreviewQuery = {
  from: string
  to: string
  sendMsg: string
}

/** Base64 inner CW20 hook for pair `Send+Swap` (classification + debit). */
export function encodePairSwapTaxPreviewSendMsg(input?: { maxSpread?: string; trader?: string | null }): string {
  const maxSpread = input?.maxSpread?.trim() || '1'
  const swap: Record<string, unknown> = { max_spread: maxSpread }
  if (input?.trader && input.trader.startsWith('terra1')) {
    swap.trader = input.trader
  }
  return btoa(JSON.stringify({ swap }))
}

/** Official-router hop: pair `Swap` hook with authenticated trader (#607 / multitest preview). */
export function encodeRouterHopSwapTaxPreviewSendMsg(trader: string, maxSpread?: string): string {
  return encodePairSwapTaxPreviewSendMsg({ maxSpread, trader })
}

export function resolveRouterSellHopPairAddress(input: {
  payToken: string
  routeOps: SwapOperation[] | null | undefined
  pairs: PairInfo[]
}): string | null {
  const ops = input.routeOps
  if (!ops?.length) return null
  for (const op of ops) {
    const offer = assetInfoLabel(op.terra_swap.offer_asset_info)
    if (offer !== input.payToken) continue
    const ask = assetInfoLabel(op.terra_swap.ask_asset_info)
    const matched = input.pairs.find((p) => {
      const a = assetInfoLabel(p.asset_infos[0])
      const b = assetInfoLabel(p.asset_infos[1])
      return (a === offer && b === ask) || (b === offer && a === ask)
    })
    if (matched?.contract_addr.startsWith('terra1')) return matched.contract_addr
  }
  return null
}

export function resolveCommunityTaxPreviewQuery(input: {
  wallet: string | null | undefined
  payToken: string
  usesRouter: boolean
  directPairAddr: string | null | undefined
  routeOps: SwapOperation[] | null | undefined
  pairs: PairInfo[]
  maxSpread?: string
}): CommunityTaxPreviewQuery | null {
  if (!input.wallet?.startsWith('terra1') || !input.payToken.startsWith('terra1')) return null
  if (input.usesRouter) {
    if (!ROUTER_CONTRACT_ADDRESS?.startsWith('terra1')) return null
    const hopPair = resolveRouterSellHopPairAddress({
      payToken: input.payToken,
      routeOps: input.routeOps,
      pairs: input.pairs,
    })
    if (!hopPair) return null
    return {
      from: ROUTER_CONTRACT_ADDRESS,
      to: hopPair,
      sendMsg: encodeRouterHopSwapTaxPreviewSendMsg(input.wallet, input.maxSpread),
    }
  }
  if (!input.directPairAddr?.startsWith('terra1')) return null
  return {
    from: input.wallet,
    to: input.directPairAddr,
    sendMsg: encodePairSwapTaxPreviewSendMsg({ maxSpread: input.maxSpread }),
  }
}
