import { assertRouteTaxFunding } from '@/services/terraclassic/routeTaxFunding'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { quoteCw20ViaRouteSolve, quoteCw20ExactOutputViaRouteSolve } from './cw20RouteSolveQuote'
import * as indexerClient from '@/services/indexer/client'
import * as router from '@/services/terraclassic/router'
import { getPair } from '@/services/terraclassic/factory'
import * as preflight from '@/services/terraclassic/swapRoutePreflight'

vi.mock('@/services/terraclassic/routeTaxFunding', () => ({
  assertRouteTaxFunding: vi.fn().mockResolvedValue(undefined),
  ROUTE_TAX_FUNDING_UNAVAILABLE: 'Route tax funding could not be verified.',
}))

vi.mock('@/services/indexer/client', () => ({
  getRouteSolve: vi.fn(),
  getRouteSolveExactOutput: vi.fn(),
}))

vi.mock('@/services/terraclassic/router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/router')>()
  return {
    ...actual,
    simulateMultiHopSwap: vi.fn(),
  }
})

vi.mock('@/services/terraclassic/factory', () => ({ getPair: vi.fn() }))

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
    vi.mocked(assertRouteTaxFunding).mockResolvedValue(undefined)
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
    expect(quoted?.executeAmountOut).toBeUndefined()
  })

  it('strips interior hybrid on GET multihop before wallet sim (#1280 Policy A)', async () => {
    const mid = 'terra1mid00000000000000000000000000000001'
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: from,
      token_out: to,
      hops: [
        { pair, offer_token: from, ask_token: mid },
        { pair: 'terra1pair2', offer_token: mid, ask_token: to },
      ],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: from } },
            ask_asset_info: { token: { contract_addr: mid } },
            hybrid: {
              pool_input: '800000',
              book_input: '200000',
              max_maker_fills: 8,
              book_start_hint: null,
            },
          },
        },
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: mid } },
            ask_asset_info: { token: { contract_addr: to } },
            hybrid: {
              pool_input: '10',
              book_input: '10',
              max_maker_fills: 8,
              book_start_hint: null,
            },
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '999999',
      intermediate_tokens: [from, mid, to],
    } as Awaited<ReturnType<typeof indexerClient.getRouteSolve>>)
    vi.mocked(router.simulateMultiHopSwap).mockResolvedValue({ amount: '950000' })

    const quoted = await quoteCw20ViaRouteSolve({
      fromToken: from,
      toToken: to,
      simRaw: '1000000',
      maxMakerFills: 8,
      slippageTolerancePercent: 5,
      maxSpreadStr: '0.05',
    })

    expect(quoted?.indexerOperations?.[0]?.terra_swap.hybrid?.book_input).toBe('200000')
    expect(quoted?.indexerOperations?.[1]?.terra_swap.hybrid).toBeUndefined()
    const simOps = vi.mocked(router.simulateMultiHopSwap).mock.calls[0][1]
    expect(simOps[1].terra_swap.hybrid).toBeUndefined()
  })

  it('shows buy-split net You Receive and keeps pre-tax execute amount (#615)', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: from,
      token_out: to,
      hops: [{ pair, offer_token: from, ask_token: to }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: from } },
            ask_asset_info: { token: { contract_addr: to } },
            hybrid: null,
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '950000',
      estimated_amount_out_net: '940500',
      buy_tax_bps: 100,
      tax_kind: 'buy',
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
    })

    expect(quoted?.return_amount).toBe('940500')
    expect(quoted?.executeAmountOut).toBe('950000')
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

describe('exact-output selected route validation (#1376)', () => {
  const from = 'terra1from'
  const to = 'terra1to'
  const request = {
    fromToken: from,
    toToken: to,
    targetRaw: '100000000000000000000',
    maxMakerFills: 8,
    slippageTolerancePercent: 5,
    maxSpreadStr: '0.05',
    quoteTrader: { trader: 'terra1trader' },
  }
  const response = () => ({
    token_in: from,
    token_out: to,
    hops: [{ pair: 'terra1pair', offer_token: from, ask_token: to }],
    router_operations: [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: from } },
          ask_asset_info: { token: { contract_addr: to } },
        },
      },
    ],
    required_amount_in: '12649685215',
    target_amount_out: request.targetRaw,
    sizing_mode: 'pool_only_exact_output' as const,
    fidelity_check: 'passed' as const,
    spot_amount_out: '100000000005328338879',
  })

  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(assertRouteTaxFunding).mockResolvedValue(undefined)
    vi.mocked(getPair).mockResolvedValue({
      contract_addr: 'terra1pair',
      liquidity_token: 'terra1lp',
      asset_infos: [{ token: { contract_addr: from } }, { token: { contract_addr: to } }],
    })
    vi.mocked(indexerClient.getRouteSolveExactOutput).mockResolvedValue(response())
    vi.mocked(router.simulateMultiHopSwap).mockResolvedValue({ amount: '100000000005328338879' })
  })

  it('keeps the realistic 100 CL8Y selected offer and operations through wallet validation', async () => {
    const quote = await quoteCw20ExactOutputViaRouteSolve(request)
    expect(quote.offerAmount).toBe('12649685215')
    expect(quote.return_amount).toBe('100000000005328338879')
    expect(indexerClient.getRouteSolveExactOutput).toHaveBeenCalledWith(
      from,
      to,
      request.targetRaw,
      expect.objectContaining({ trader: 'terra1trader' })
    )
    expect(router.simulateMultiHopSwap).toHaveBeenCalledWith(
      quote.offerAmount,
      quote.indexerOperations,
      request.quoteTrader
    )
    expect(indexerClient.getRouteSolve).not.toHaveBeenCalled()
  })

  it('validates buy-tax net while preserving the pre-tax contract minimum base', async () => {
    vi.mocked(indexerClient.getRouteSolveExactOutput).mockResolvedValue({
      ...response(),
      buy_tax_bps: 1000,
      spot_amount_out: '111111111111111111111',
    })
    vi.mocked(router.simulateMultiHopSwap).mockResolvedValue({ amount: '111111111111111111111' })
    const quote = await quoteCw20ExactOutputViaRouteSolve(request)
    expect(BigInt(quote.return_amount)).toBeGreaterThanOrEqual(BigInt(request.targetRaw))
    expect(quote.executeAmountOut).toBe('111111111111111111111')
  })

  it.each([undefined, '0', '-1', '1e20', (1n << 128n).toString(), '9'.repeat(1000)])(
    'rejects missing or malformed fair reference %s before wallet work',
    async (spot) => {
      vi.mocked(indexerClient.getRouteSolveExactOutput).mockResolvedValue({ ...response(), spot_amount_out: spot })
      await expect(quoteCw20ExactOutputViaRouteSolve(request)).rejects.toThrow('fair price reference')
      expect(getPair).not.toHaveBeenCalled()
      expect(router.simulateMultiHopSwap).not.toHaveBeenCalled()
    }
  )

  it('retains symmetric fair-price deviation for the unchanged hard retail protections', async () => {
    vi.mocked(indexerClient.getRouteSolveExactOutput).mockResolvedValue({
      ...response(),
      spot_amount_out: '100000000005328338879000',
    })
    const quote = await quoteCw20ExactOutputViaRouteSolve(request)
    expect(quote.routeSlippagePercent).toBe('99.90')
  })

  it('rejects insufficient fresh wallet output instead of trusting the server estimate', async () => {
    vi.mocked(router.simulateMultiHopSwap).mockResolvedValue({ amount: '99999999999999999999' })
    await expect(quoteCw20ExactOutputViaRouteSolve(request)).rejects.toThrow('cannot satisfy')
  })

  it.each(['drift', 'target', 'overflow', 'hybrid', 'disconnected', 'endpoint', 'intermediary', 'hop-limit'])(
    'rejects forged or stale %s responses before wallet work',
    async (kind) => {
      const value = response()
      if (kind === 'drift') value.fidelity_check = 'drift' as 'passed'
      if (kind === 'target') value.target_amount_out = '1'
      if (kind === 'overflow') value.required_amount_in = (1n << 128n).toString()
      if (kind === 'hybrid')
        Object.assign(value.router_operations[0].terra_swap, { hybrid: { pool_input: '1', book_input: '1' } })
      if (kind === 'endpoint') value.hops[0].ask_token = 'terra1wrong'
      if (kind === 'intermediary') value.hops.push({ pair: 'terra1pair2', offer_token: 'terra1wrong', ask_token: to })
      if (kind === 'hop-limit') value.hops = Array(5).fill(value.hops[0])
      if (kind === 'disconnected')
        value.router_operations[0].terra_swap.offer_asset_info.token.contract_addr = 'terra1wrong'
      vi.mocked(indexerClient.getRouteSolveExactOutput).mockResolvedValue(value)
      await expect(quoteCw20ExactOutputViaRouteSolve(request)).rejects.toThrow()
      expect(router.simulateMultiHopSwap).not.toHaveBeenCalled()
    }
  )
})

describe('all-tax net route quotes (#1381)', () => {
  it('does not haircut an already-net router result and keeps gross input for one hop', async () => {
    vi.mocked(assertRouteTaxFunding).mockResolvedValue(1)
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: 'terra1alpha',
      token_out: 'terra1ask',
      tax_kind: 'route_local_v1',
      buy_tax_bps: 450,
      hops: [{ pair: 'terra1pair', offer_token: 'terra1alpha', ask_token: 'terra1ask' }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: 'terra1alpha' } },
            ask_asset_info: { token: { contract_addr: 'terra1ask' } },
          },
        },
      ],
    })
    vi.mocked(router.simulateMultiHopSwap).mockResolvedValue({ amount: '955000', taxFundingVersion: 1 })
    const result = await quoteCw20ViaRouteSolve({
      fromToken: 'terra1alpha',
      toToken: 'terra1ask',
      simRaw: '1010000',
      maxMakerFills: 8,
      slippageTolerancePercent: 1,
      maxSpreadStr: '0.01',
    })
    expect(result).toMatchObject({ return_amount: '955000', taxFundingVersion: 1 })
    expect(result?.executeAmountOut).toBeUndefined()
    expect(router.simulateMultiHopSwap).toHaveBeenLastCalledWith(
      '1010000',
      expect.any(Array),
      expect.objectContaining({ taxFundingVersion: 1 })
    )
  })
})

describe('authorized route identity (#1381)', () => {
  it('rejects a versioned quote whose executable path pays a different receive token', async () => {
    vi.mocked(assertRouteTaxFunding).mockClear()
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: 'terra1offer',
      token_out: 'terra1wanted',
      tax_kind: 'route_local_v1',
      hops: [{ pair: 'terra1pair', offer_token: 'terra1offer', ask_token: 'terra1foreign' }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: 'terra1offer' } },
            ask_asset_info: { token: { contract_addr: 'terra1foreign' } },
          },
        },
      ],
    })
    await expect(
      quoteCw20ViaRouteSolve({
        fromToken: 'terra1offer',
        toToken: 'terra1wanted',
        simRaw: '1010000',
        maxMakerFills: 8,
        slippageTolerancePercent: 1,
        maxSpreadStr: '0.01',
      })
    ).rejects.toThrow('receive token does not match')
    expect(assertRouteTaxFunding).not.toHaveBeenCalled()
  })
  it('rejects disconnected funded operation segments before trusting a capability', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockResolvedValue({
      token_in: 'terra1offer',
      token_out: 'terra1wanted',
      tax_kind: 'route_local_v1',
      hops: [
        { pair: 'terra1pair1', offer_token: 'terra1offer', ask_token: 'terra1middle' },
        { pair: 'terra1pair2', offer_token: 'terra1foreign', ask_token: 'terra1wanted' },
      ],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: 'terra1offer' } },
            ask_asset_info: { token: { contract_addr: 'terra1middle' } },
          },
        },
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: 'terra1foreign' } },
            ask_asset_info: { token: { contract_addr: 'terra1wanted' } },
          },
        },
      ],
    })
    await expect(
      quoteCw20ViaRouteSolve({
        fromToken: 'terra1offer',
        toToken: 'terra1wanted',
        simRaw: '1010000',
        maxMakerFills: 8,
        slippageTolerancePercent: 1,
        maxSpreadStr: '0.01',
      })
    ).rejects.toThrow('operations do not match')
  })
})
