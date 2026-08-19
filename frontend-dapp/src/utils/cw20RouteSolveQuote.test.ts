import { describe, it, expect, vi, beforeEach } from 'vitest'
import { quoteCw20ViaRouteSolve } from './cw20RouteSolveQuote'
import * as indexerClient from '@/services/indexer/client'
import * as router from '@/services/terraclassic/router'
import * as preflight from '@/services/terraclassic/swapRoutePreflight'

vi.mock('@/services/indexer/client', () => ({
  getRouteSolve: vi.fn(),
}))

vi.mock('@/services/terraclassic/router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/router')>()
  return {
    ...actual,
    simulateMultiHopSwap: vi.fn(),
  }
})

vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  enrichSwapOperationsWithHopMinReturns: vi.fn(async (operations: unknown[]) => operations),
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.10',
    anyHopExceedsMaxSpread: false,
  }),
}))

describe('quoteCw20ViaRouteSolve (#501)', () => {
  const from = 'terra1from00000000000000000000000000000001'
  const to = 'terra1to00000000000000000000000000000001'
  const pair = 'terra1pair00000000000000000000000000000001'

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('quotes via GET /route/solve and wallet simulate_swap_operations', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: from,
      token_out: to,
      hops: [{ pair, offer_token: from, ask_token: to }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: from } },
            ask_asset_info: { token: { contract_addr: to } },
            hybrid: {
              pool_input: '800000',
              book_input: '200000',
              max_maker_fills: 8,
              book_start_hint: null,
            },
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '999999',
      spot_amount_out: '1000000',
      slippage_percent: '1.00',
      intermediate_tokens: [from, to],
    } as Awaited<ReturnType<typeof indexerClient.getRouteSolve>>)
    vi.mocked(router.simulateMultiHopSwap).mockResolvedValue({ amount: '950000' })

    const quoted = await quoteCw20ViaRouteSolve({
      fromToken: from,
      toToken: to,
      simRaw: '1000000',
      maxMakerFills: 8,
      slippageTolerancePercent: 5,
      maxSpreadStr: '0.05',
      quoteTrader: { trader: 'terra1wallet000000000000000000000000001' },
    })

    expect(indexerClient.getRouteSolve).toHaveBeenCalledWith(
      from,
      to,
      '1000000',
      expect.objectContaining({ maxMakerFills: 8, trader: 'terra1wallet000000000000000000000000001' })
    )
    expect(router.simulateMultiHopSwap).toHaveBeenCalled()
    expect(preflight.enrichSwapOperationsWithHopMinReturns).toHaveBeenCalled()
    expect(quoted).toMatchObject({
      return_amount: '950000',
      indexerQuoteKind: 'indexer_hybrid_lcd',
    })
    expect(quoted?.indexerOperations?.[0]?.terra_swap.hybrid).toEqual({
      pool_input: '800000',
      book_input: '200000',
      max_maker_fills: 8,
      book_start_hint: null,
    })
    // Wallet amount wins over indexer estimated_amount_out for receive.
    expect(quoted?.return_amount).toBe('950000')
  })

  it('returns null when indexer token_in/out disagree with the request', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: from,
      token_out: 'terra1other000000000000000000000000000001',
      hops: [],
      router_operations: [],
    } as Awaited<ReturnType<typeof indexerClient.getRouteSolve>>)

    const quoted = await quoteCw20ViaRouteSolve({
      fromToken: from,
      toToken: to,
      simRaw: '1000000',
      maxMakerFills: 8,
      slippageTolerancePercent: 5,
      maxSpreadStr: '0.05',
    })

    expect(quoted).toBeNull()
    expect(router.simulateMultiHopSwap).not.toHaveBeenCalled()
  })

  it('returns null when a production economic quote hops a gem (GitLab #562 A10)', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    const ruby = 'terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc'
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: from,
      token_out: to,
      hops: [
        { pair, offer_token: from, ask_token: ruby },
        { pair: 'terra1pair2', offer_token: ruby, ask_token: to },
      ],
      router_operations: [],
      intermediate_tokens: [from, ruby, to],
    } as Awaited<ReturnType<typeof indexerClient.getRouteSolve>>)

    const quoted = await quoteCw20ViaRouteSolve({
      fromToken: from,
      toToken: to,
      simRaw: '1000000',
      maxMakerFills: 8,
      slippageTolerancePercent: 5,
      maxSpreadStr: '0.05',
    })

    expect(quoted).toBeNull()
    expect(router.simulateMultiHopSwap).not.toHaveBeenCalled()
    vi.unstubAllEnvs()
  })
})
