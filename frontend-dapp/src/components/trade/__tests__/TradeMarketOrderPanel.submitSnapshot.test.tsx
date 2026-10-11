import { useWalletStore } from '@/hooks/useWallet'
import { executeMultiHopSwap } from '@/services/terraclassic/router'
import { renderWithProviders } from '@/test-utils'
import { LOCAL_WALLET, localQuote, localTokens } from '@/test/localRouteFixtures'
import type { PairInfo } from '@/types'
import { fireEvent, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { TradeMarketOrderPanel } from '../TradeMarketOrderPanel'
vi.mock('@/services/terraclassic/routeTaxFunding', () => ({
  assertRouteTaxFunding: vi.fn().mockResolvedValue(undefined),
  queryRouterTaxFundingVersion: vi.fn().mockResolvedValue(undefined),
  ROUTE_TAX_FUNDING_UNAVAILABLE: 'Route tax funding could not be verified.',
}))

const PAIR_ADDR = 'terra1pair00000000000000000000000000000001'
const TERRA_A = 'terra1from00000000000000000000000000000001'
const TERRA_B = 'terra1to00000000000000000000000000000001'

const selectedPair: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra1lp000000000000000000000000000000001',
  asset_infos: [{ token: { contract_addr: TERRA_A } }, { token: { contract_addr: TERRA_B } }],
}

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue({}),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(async (_addr: string, msg: unknown) => {
    if (msg && typeof msg === 'object' && 'token_info' in msg) {
      return { name: 'Dummy', symbol: 'DUM', decimals: 6, total_supply: '0' }
    }
    return {}
  }),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/hooks/useLimitOrderEscrowBalance', () => ({
  useLimitOrderEscrowBalance: () => ({ data: '10000000000', isLoading: false, isError: false }),
}))

vi.mock('@/hooks/useNativeUlunaBalance', () => ({
  useNativeUlunaBalance: () => ({ data: '10000000000', isLoading: false, isError: false }),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  simulateSwap: vi.fn().mockResolvedValue({
    return_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  simulateHybridSwap: vi.fn().mockResolvedValue({
    return_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  swap: vi.fn().mockResolvedValue('txhash'),
}))

vi.mock('@/services/terraclassic/router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/router')>()
  return {
    ...actual,
    simulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '1000000' }),
    executeMultiHopSwap: vi.fn().mockResolvedValue('txhash'),
  }
})

vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
  enrichSwapOperationsWithHopMinReturns: vi.fn(async (operations: unknown[]) => operations),
  computeDirectHybridMinReturn: vi.fn().mockResolvedValue('900000'),
}))

vi.mock('@/services/terraclassic/transactions', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/terraclassic/transactions')>()),
  executeCw20AllowanceThen: vi.fn(async (_a, _b, _c, _d, fn: () => Promise<string>) => fn()),
  estimateMarketPairSwapSequenceUlunaFeesTotal: vi.fn().mockReturnValue(1000000n),
}))

vi.mock('@/services/indexer/client', () => {
  const from = 'terra1from00000000000000000000000000000001'
  const to = 'terra1to00000000000000000000000000000001'
  const pair = 'terra1pair00000000000000000000000000000001'
  return {
    getRouteSolve: vi.fn().mockResolvedValue({
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
      estimated_amount_out: '1000000',
      spot_amount_out: '1010000',
      slippage_percent: '1.00',
      intermediate_tokens: [from, to],
    }),
    postRouteSolve: vi.fn().mockRejectedValue(new Error('indexer unavailable')),
    getTokens: vi.fn().mockResolvedValue([]),
    getRouteSolveProgress: vi.fn().mockResolvedValue({
      stage: 'idle',
      done: 1,
      total: 1,
      label: 'Local route ready',
      cache_hit: false,
      updated_at_ms: Date.now(),
      quote_generation: 'fixture1',
    }),
  }
})

import * as indexerClient from '@/services/indexer/client'
import { parseRouteQuoteError } from '@/utils/routeQuoteErrors'
import * as pair from '@/services/terraclassic/pair'
import { simulateMultiHopSwap } from '@/services/terraclassic/router'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

async function openAdvanced(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByTestId('trade-market-advanced-toggle'))
}

describe('TradeMarketOrderPanel local submit snapshot (#1452)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(indexerClient.postRouteSolve).mockRejectedValue(new Error('indexer unavailable'))
    useWalletStore.setState({ address: LOCAL_WALLET, walletType: 'simulated', error: null })
    vi.mocked(indexerClient.getTokens).mockResolvedValue(localTokens)
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) =>
      localQuote(from, to, amount, options)
    )
  })
  async function ready() {
    const rendered = renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )
    fireEvent.change(screen.getByTestId('limit-order-escrow-amount-input'), { target: { value: '1' } })
    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())
    return rendered
  }
  it('quotes the default through local GET and the actual selected spread', async () => {
    await ready()
    expect(indexerClient.getRouteSolve).toHaveBeenCalledWith(
      TERRA_A,
      TERRA_B,
      '1000000',
      expect.objectContaining({ maxSpread: '0.05', trader: LOCAL_WALLET })
    )
    expect(pair.simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
    expect(indexerClient.postRouteSolve).not.toHaveBeenCalled()
  })
  it('signs precisely the reviewed single-hop router operations with positive floors', async () => {
    await ready()
    await userEvent.click(screen.getByTestId('trade-market-submit'))
    await waitFor(() => expect(executeMultiHopSwap).toHaveBeenCalled())
    expect(vi.mocked(executeMultiHopSwap).mock.calls[0]).toEqual([
      LOCAL_WALLET,
      TERRA_A,
      '1000000',
      expect.arrayContaining([
        expect.objectContaining({ terra_swap: expect.objectContaining({ min_return: '950000' }) }),
      ]),
      '0.05',
      '950000',
      undefined,
      expect.any(Number),
      expect.objectContaining({ generation: 'fixture1' }),
    ])
    expect(pair.swap).not.toHaveBeenCalled()
  })
  it('keeps the pay field stale until the correct raw quote arrives', async () => {
    await ready()
    fireEvent.change(screen.getByTestId('limit-order-escrow-amount-input'), { target: { value: '10' } })
    expect(screen.getByTestId('trade-market-submit')).toBeDisabled()
    await waitFor(() =>
      expect(indexerClient.getRouteSolve).toHaveBeenCalledWith(TERRA_A, TERRA_B, '10000000', expect.anything())
    )
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())
  })
  it('fails closed when local observations for a manual split are unavailable', async () => {
    await ready()
    await openAdvanced(userEvent.setup())
    fireEvent.change(screen.getByTestId('trade-market-book-leg-input'), { target: { value: '0.1' } })
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeDisabled())
    await waitFor(() =>
      expect(screen.getByTestId('trade-market-quote-error')).toHaveTextContent(/temporarily unavailable/i)
    )
    expect(indexerClient.postRouteSolve).toHaveBeenCalledWith(
      TERRA_A,
      TERRA_B,
      '1000000',
      [{ pool_input: '900000', book_input: '100000', max_maker_fills: 8, book_start_hint: null }],
      expect.objectContaining({ maxSpread: '0.05', trader: LOCAL_WALLET })
    )
    expect(pair.simulateHybridSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
  })
  it('pins and signs the exact locally admitted manual split', async () => {
    vi.mocked(indexerClient.postRouteSolve).mockImplementation(async (from, to, amount, plan, options) => {
      const q = localQuote(from, to, amount, options)
      const split = plan[0]!
      const h = q.execution_metadata!.hops[0]
      Object.assign(h, {
        pool_input_actual: split.pool_input,
        book_offer_consumed: split.book_input,
        pool_return_amount: split.pool_input,
        book_return_net: split.book_input,
      })
      h.declared_hybrid = { ...split, book_start_hint: null }
      q.execution_metadata!.policy = 'declared_hybrid_manual'
      ;(q.router_operations[0] as import('@/services/terraclassic/router').SwapOperation).terra_swap.hybrid = {
        ...split,
        book_start_hint: null,
      }
      return q
    })
    await ready()
    await openAdvanced(userEvent.setup())
    fireEvent.change(screen.getByTestId('trade-market-book-leg-input'), { target: { value: '0.1' } })
    await waitFor(() => expect(indexerClient.postRouteSolve).toHaveBeenCalled())
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())
    fireEvent.click(screen.getByTestId('trade-market-submit'))
    await waitFor(() => expect(executeMultiHopSwap).toHaveBeenCalled())
    const call = vi.mocked(executeMultiHopSwap).mock.calls.at(-1)!
    expect(call[2]).toBe('1000000')
    expect(call[3][0].terra_swap.hybrid).toMatchObject({
      pool_input: '900000',
      book_input: '100000',
      max_maker_fills: 8,
    })
    expect(call[3][0].terra_swap.min_return).toBe('950000')
    expect(call[5]).toBe('950000')
    expect(call[8]).toMatchObject({ policy: 'declared_hybrid_manual', generation: 'fixture1' })
    expect(pair.simulateHybridSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
  })
  it('retains a useful receive through failed refresh with submit blocked', async () => {
    const { queryClient } = await ready()
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new Error('Indexer API error: 503'))
    await queryClient.invalidateQueries({ queryKey: ['tradeMarketSim'] })
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeDisabled())
    expect(screen.getByTestId('trade-market-expected-receive')).toHaveTextContent('1')
  })
  it('keeps compact signing anchors and no hybrid opt-out', async () => {
    await ready()
    await openAdvanced(userEvent.setup())
    expect(screen.queryByTestId('trade-market-hybrid-toggle')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-confirm-action')).toHaveTextContent('Market swap')
    expect(screen.getByTestId('swap-confirm-offer')).toHaveTextContent('1')
    expect(screen.getByTestId('swap-confirm-min-return')).toHaveTextContent('0.95')
    expect(screen.getByTestId('swap-confirm-max-spread')).toHaveTextContent('5%')
  })
  it('never quotes a live fallback after a deterministic route rejection', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new Error('Indexer API error: 422 Unprocessable Entity'))
    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )
    fireEvent.change(screen.getByTestId('limit-order-escrow-amount-input'), { target: { value: '1' } })
    await screen.findByTestId('trade-market-quote-error')
    expect(screen.getByTestId('trade-market-submit')).toBeDisabled()
    expect(pair.simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
  })
  it.each(['ask', 'bid'] as const)(
    'keeps %s protection estimates unsigned and recovers after an amount edit (#1473)',
    async (side) => {
      vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
        if (amount === '100000000')
          throw parseRouteQuoteError(
            422,
            JSON.stringify({
              error: 'protection',
              execution_supported: false,
              indicative_amount_out: '140000000',
              generation: 'fixture1',
              observed_at_ms: Date.now() - 1000,
              expires_at_ms: Date.now() + 19_000,
            })
          )
        return localQuote(from, to, amount, options)
      })
      renderWithProviders(
        <TradeMarketOrderPanel
          pairAddr={PAIR_ADDR}
          selectedPair={selectedPair}
          pairs={[selectedPair]}
          side={side}
          isPaused={false}
        />
      )
      fireEvent.change(screen.getByTestId('limit-order-escrow-amount-input'), { target: { value: '100' } })
      await screen.findByTestId('trade-market-quote-error')
      expect(screen.getByTestId('trade-market-quote-error')).toHaveTextContent('selected price protection')
      expect(screen.queryByTestId('trade-market-indicative-estimate')).not.toBeInTheDocument()
      expect(screen.queryByText(/Estimate only/)).not.toBeInTheDocument()
      expect(screen.queryByTestId('trade-market-expected-receive')).not.toBeInTheDocument()
      expect(screen.getByTestId('trade-market-submit')).toBeDisabled()
      fireEvent.click(screen.getByTestId('trade-market-submit'))
      expect(executeMultiHopSwap).not.toHaveBeenCalled()
      fireEvent.change(screen.getByTestId('limit-order-escrow-amount-input'), { target: { value: '1' } })
      expect(screen.queryByTestId('trade-market-quote-error')).not.toBeInTheDocument()
      await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())
      expect(screen.getByTestId('trade-market-expected-receive')).toHaveTextContent('1')
      expect(pair.simulateSwap).not.toHaveBeenCalled()
      expect(simulateMultiHopSwap).not.toHaveBeenCalled()
    }
  )
})
