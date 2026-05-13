import { describe, it, expect } from 'vitest'
import {
  computeSwapRouteDisplay,
  tokenPathFromSwapOperations,
  tokenPathForNativeSupportedRoute,
} from './swapRouteDisplay'
import type { SwapOperation } from '@/services/terraclassic/router'

const sym = (id: string) => (id.startsWith('terra1') ? id.slice(-4) : id)

const op = (offer: string, ask: string): SwapOperation => ({
  terra_swap: {
    offer_asset_info: { token: { contract_addr: offer } },
    ask_asset_info: { token: { contract_addr: ask } },
  },
})

describe('swapRouteDisplay', () => {
  it('tokenPathFromSwapOperations chains offer/ask', () => {
    expect(tokenPathFromSwapOperations([op('A', 'B'), op('B', 'C')])).toEqual(['A', 'B', 'C'])
  })

  it('prefers indexer path when indexerOperations are present (GitLab #158)', () => {
    const from = 'terra1from00000000000000000000000000000001'
    const to = 'terra1to00000000000000000000000000000001'
    const mid = 'terra1mid00000000000000000000000000000001'
    const client: SwapOperation[] = [op(from, mid), op(mid, to)]
    const line = computeSwapRouteDisplay({
      fromToken: from,
      toToken: to,
      isWrapOrUnwrap: false,
      nativeRouteInfo: null,
      indexerIntermediateTokens: [from, mid, to],
      indexerOperations: client,
      clientRoute: client,
      isMultiHop: true,
      isDirect: false,
      displaySymbol: sym,
    })
    expect(line).toBe(`${sym(from)} → ${sym(mid)} → ${sym(to)}`)
  })

  it('uses client graph only when indexer operations are absent', () => {
    const from = 'terra1aa0000000000000000000000000000000001'
    const mid = 'terra1bb0000000000000000000000000000000001'
    const to = 'terra1cc0000000000000000000000000000000001'
    const client: SwapOperation[] = [op(from, mid), op(mid, to)]
    const line = computeSwapRouteDisplay({
      fromToken: from,
      toToken: to,
      isWrapOrUnwrap: false,
      nativeRouteInfo: null,
      indexerIntermediateTokens: undefined,
      indexerOperations: undefined,
      clientRoute: client,
      isMultiHop: true,
      isDirect: false,
      displaySymbol: sym,
    })
    expect(line).toBe(`${sym(from)} → ${sym(mid)} → ${sym(to)}`)
  })

  it('tokenPathForNativeSupportedRoute prepends native input when wrapping', () => {
    const from = 'uluna'
    const to = 'terra1w0000000000000000000000000000000001'
    const line = tokenPathForNativeSupportedRoute(
      from,
      to,
      {
        operations: [op('terra1w0000000000000000000000000000000001', 'terra1x0000000000000000000000000000000001')],
        needsWrapInput: true,
        needsUnwrapOutput: false,
      },
      sym
    )
    expect(line[0]).toBe(sym(from))
    expect(line[line.length - 1]).toBe(sym('terra1x0000000000000000000000000000000001'))
  })
})
