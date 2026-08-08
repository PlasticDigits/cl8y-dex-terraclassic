import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { TradeMarketOrderPanel } from '../TradeMarketOrderPanel'
import { useWalletStore } from '@/hooks/useWallet'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import type { PairInfo } from '@/types'

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

vi.mock('@/services/terraclassic/transactions', () => ({
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
  }
})

import * as pair from '@/services/terraclassic/pair'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

async function openAdvanced(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByTestId('trade-market-advanced-toggle'))
}

describe('TradeMarketOrderPanel submit snapshot (GitLab #360 / #501)', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.clearAllMocks()
    useWalletStore.setState({
      address: 'terra1wallet000000000000000000000000001',
      walletType: 'simulated',
      error: null,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('defaults to GET /route/solve (not POST) when hybrid on and book leg empty (#501)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)

    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    expect(indexerClient.postRouteSolve).not.toHaveBeenCalled()
    expect(await screen.findByTestId('trade-market-quote')).toHaveTextContent(/limit book \+ pool/i)
  })

  it('hybrid off skips GET/POST route/solve and uses pool-only simulateSwap (#501)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    await openAdvanced(user)
    await user.click(screen.getByTestId('trade-market-hybrid-toggle'))
    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)

    await waitFor(() => expect(pair.simulateSwap).toHaveBeenCalled())
    expect(indexerClient.getRouteSolve).not.toHaveBeenCalled()
    expect(indexerClient.postRouteSolve).not.toHaveBeenCalled()
    const quoteCard = await screen.findByTestId('trade-market-quote')
    expect(quoteCard).toHaveTextContent(/pool/i)
    expect(quoteCard).not.toHaveTextContent(/limit book \+ pool/i)
  })

  it('submits GET solver hybrid with max_maker_fills submit cap (#501 / #249)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())

    await user.click(screen.getByTestId('trade-market-submit'))
    await waitFor(() => expect(pair.swap).toHaveBeenCalled())
    const swapArgs = vi.mocked(pair.swap).mock.calls.at(-1)
    expect(swapArgs?.[7]?.hybrid).toEqual({
      pool_input: '800000',
      book_input: '200000',
      max_maker_fills: 8,
      book_start_hint: null,
    })
  })

  it('uses POST /route/solve only for Advanced manual book leg override (#501)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    vi.mocked(indexerClient.postRouteSolve).mockResolvedValue({
      token_in: TERRA_A,
      token_out: TERRA_B,
      hops: [{ pair: PAIR_ADDR, offer_token: TERRA_A, ask_token: TERRA_B }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: TERRA_A } },
            ask_asset_info: { token: { contract_addr: TERRA_B } },
            hybrid: {
              pool_input: '500000',
              book_input: '500000',
              max_maker_fills: 8,
              book_start_hint: null,
            },
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '1000000',
    } as Awaited<ReturnType<typeof indexerClient.postRouteSolve>>)

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await openAdvanced(user)
    await user.type(screen.getByTestId('trade-market-book-leg-input'), '0.5')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)

    await waitFor(() => expect(indexerClient.postRouteSolve).toHaveBeenCalled())
  })

  it('disables market submit with quoting state while book leg differs from debounced hybrid quote', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    const amountInput = screen.getByTestId('limit-order-escrow-amount-input')
    await user.type(amountInput, '10')
    await openAdvanced(user)

    const bookInput = screen.getByTestId('trade-market-book-leg-input')
    await user.type(bookInput, '2')

    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())

    await user.clear(bookInput)
    await user.type(bookInput, '5')

    expect(screen.getByTestId('trade-market-submit')).toBeDisabled()

    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())
  })

  it('shows labeled pre-sign confirmation fields before market swap submit (GitLab #409 / SEC-D11)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)

    expect(await screen.findByTestId('trade-market-pre-submit-summary')).toBeInTheDocument()
    expect(screen.getByTestId('swap-confirm-action')).toHaveTextContent('Market swap')
    expect(screen.getByTestId('swap-confirm-pair')).toHaveTextContent('→')
    expect(screen.getByTestId('swap-confirm-offer')).toHaveTextContent('1')
    expect(screen.getByTestId('swap-confirm-receive')).toHaveTextContent('1')
    expect(screen.getByTestId('swap-confirm-max-spread')).toHaveTextContent('5%')
    expect(screen.getByTestId('swap-confirm-min-return')).toHaveTextContent('0.95')
    expect(screen.getByTestId('swap-confirm-chain')).toHaveTextContent('LocalTerra')
  })

  it('shows retail-friendly disclosure on the market quote card (#414)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)

    const quoteCard = await screen.findByTestId('trade-market-quote')
    expect(quoteCard).toHaveTextContent(/Estimated output from limit book \+ pool/i)
    expect(quoteCard).not.toHaveTextContent(/Pattern C|hybrid_simulation|GitLab #/i)
  })

  it('surfaces hybrid min return copy in Advanced when hybrid is on (#419 / #501)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    expect(screen.queryByTestId('trade-market-hybrid-min-return-notice')).not.toBeInTheDocument()
    await openAdvanced(user)
    expect(screen.getByTestId('trade-market-hybrid-min-return-notice')).toHaveTextContent(/min return/i)
    expect(screen.getByTestId('trade-market-hybrid-min-return-notice')).toHaveTextContent(/indexer solver/i)
  })

  it('humanizes simulated quote failures instead of raw error text (#414)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new Error('indexer unavailable'))
    vi.mocked(indexerClient.postRouteSolve).mockRejectedValue(new Error('indexer unavailable'))
    vi.mocked(pair.simulateHybridSwap).mockRejectedValue(new Error('lcd fail'))
    vi.mocked(pair.simulateSwap).mockRejectedValue(new Error('lcd fail'))

    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)

    const err = await screen.findByTestId('trade-market-quote-error')
    expect(err).toHaveTextContent(/reach the chain/i)
    expect(err).not.toHaveTextContent(/lcd fail/i)
  })
})
