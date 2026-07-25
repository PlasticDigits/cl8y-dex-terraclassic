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

vi.mock('@/services/indexer/client', () => ({
  postRouteSolve: vi.fn().mockRejectedValue(new Error('indexer unavailable')),
}))

import * as pair from '@/services/terraclassic/pair'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

describe('TradeMarketOrderPanel submit snapshot (GitLab #360)', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    useWalletStore.setState({
      address: 'terra1wallet000000000000000000000000001',
      walletType: 'simulated',
      error: null,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
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

    const bookInput = screen.getByPlaceholderText('Empty = full book leg')
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

  it('surfaces hybrid min return copy before market submit (#419)', () => {
    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )

    expect(screen.getByTestId('trade-market-hybrid-min-return-notice')).toHaveTextContent(/min return/i)
    expect(screen.getByTestId('trade-market-hybrid-min-return-notice')).toHaveTextContent(/book first/i)
  })

  it('humanizes simulated quote failures instead of raw error text (#414)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
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
