import { describe, it, expect } from 'vitest'
import {
  computeSwapRouteDisplay,
  deriveSwapSubmitRouteSource,
  tokenPathFromSwapOperations,
  tokenPathForNativeSupportedRoute,
  swapRouteIntermediateTokensAligned,
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

  it('deriveSwapSubmitRouteSource matches swapMutation branches (GitLab #329)', () => {
    const from = 'terra1aa0000000000000000000000000000000001'
    const mid = 'terra1bb0000000000000000000000000000000001'
    const to = 'terra1cc0000000000000000000000000000000001'
    const client: SwapOperation[] = [op(from, mid), op(mid, to)]

    expect(
      deriveSwapSubmitRouteSource({
        isWrapOrUnwrap: false,
        nativeRouteInfo: null,
        indexerOperations: client,
        clientRoute: client,
        isDirect: false,
        isMultiHop: true,
      })
    ).toBe('indexer')

    expect(
      deriveSwapSubmitRouteSource({
        isWrapOrUnwrap: false,
        nativeRouteInfo: null,
        indexerOperations: undefined,
        clientRoute: client,
        isDirect: false,
        isMultiHop: true,
      })
    ).toBe('client_bfs')

    expect(
      deriveSwapSubmitRouteSource({
        isWrapOrUnwrap: false,
        nativeRouteInfo: null,
        indexerOperations: undefined,
        clientRoute: [op(from, to)],
        isDirect: true,
        isMultiHop: false,
      })
    ).toBe('direct')
  })

  // GitLab #450 (SEC-I02 H09): displayed intermediate tokens must match submitted operations.
  describe('swapRouteIntermediateTokensAligned', () => {
    const from = 'terra1from00000000000000000000000000000001'
    const mid = 'terra1mid000000000000000000000000000000001'
    const to = 'terra1to0000000000000000000000000000000001'
    const evil = 'terra1evil00000000000000000000000000000001'

    it('returns true when the submitted ops match the displayed path', () => {
      const ops: SwapOperation[] = [op(from, mid), op(mid, to)]
      expect(swapRouteIntermediateTokensAligned(ops, [from, mid, to])).toBe(true)
    })

    it('is case-insensitive on addresses', () => {
      const ops: SwapOperation[] = [op(from, mid), op(mid, to)]
      expect(
        swapRouteIntermediateTokensAligned(ops, [from.toUpperCase(), mid, to])
      ).toBe(true)
    })

    it('returns false when an intermediate hop is substituted', () => {
      const ops: SwapOperation[] = [op(from, evil), op(evil, to)]
      // Display claims from→mid→to but ops actually route through `evil`.
      expect(swapRouteIntermediateTokensAligned(ops, [from, mid, to])).toBe(false)
    })

    it('returns false when the displayed path length differs from the ops path', () => {
      const ops: SwapOperation[] = [op(from, to)]
      expect(swapRouteIntermediateTokensAligned(ops, [from, mid, to])).toBe(false)
    })

    it('returns false when a terminal token is substituted', () => {
      const ops: SwapOperation[] = [op(from, mid), op(mid, evil)]
      expect(swapRouteIntermediateTokensAligned(ops, [from, mid, to])).toBe(false)
    })

    it('returns true (nothing to cross-check) when ops or intermediates are absent', () => {
      expect(swapRouteIntermediateTokensAligned(undefined, [from, mid, to])).toBe(true)
      expect(swapRouteIntermediateTokensAligned([op(from, to)], undefined)).toBe(true)
      expect(swapRouteIntermediateTokensAligned([], [from, to])).toBe(true)
    })
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
