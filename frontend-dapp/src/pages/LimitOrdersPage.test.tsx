import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import LimitOrdersPage from './LimitOrdersPage'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'
import * as factory from '@/services/terraclassic/factory'
import * as indexerClient from '@/services/indexer/client'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { getPairPaused, updateLimitOrderPrice } from '@/services/terraclassic/pair'
import type { IndexerPair } from '@/types'

const PAIR = 'terra1pair0000000000000000000000000000000001'
const MAKER = 'terra1maker000000000000000000000000000001'

const mockIndexerPair: IndexerPair = {
  pair_address: PAIR,
  asset_0: { symbol: 'AAA', contract_addr: 'terra1aaa0000000000000000000000000000001', denom: null, decimals: 6 },
  asset_1: { symbol: 'BBB', contract_addr: 'terra1bbb0000000000000000000000000000002', denom: null, decimals: 6 },
  lp_token: 'terra1lp000000000000000000000000000000001',
  fee_bps: 30,
  volume_quote_24h: '0',
  is_active: true,
}

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn(),
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getPairs: vi.fn(),
    getPair: vi.fn(),
    getTrades: vi.fn(),
    getPairLimitBookPage: vi.fn(),
    getPairLimitPlacements: vi.fn(),
    getTraderLimitPlacements: vi.fn(),
    getPairLimitCancellations: vi.fn(),
  }
})

vi.mock('@/services/terraclassic/pair', () => ({
  getPairPaused: vi.fn().mockResolvedValue({ paused: false }),
  placeLimitOrderWithAllowance: vi.fn(),
  cancelLimitOrder: vi.fn(),
  updateLimitOrderPrice: vi.fn().mockResolvedValue('tx-update-price'),
  getPool: vi.fn().mockResolvedValue({ assets: [{ amount: '1000000' }, { amount: '3000000' }] }),
}))

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue(null),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({ fee_bps: 30, treasury: 'terra1treasury0000000000000000000001' }),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/hooks/useLimitOrderMakerFeeRates', () => ({
  useLimitOrderMakerFeeRates: () => ({
    effectiveFeeBps: 30,
    makerPlacementFeeBps: 30,
    feeLoading: false,
    feeError: null,
  }),
}))

vi.mock('@/hooks/usePairLimitCancellations', () => ({
  usePairLimitCancellations: () => ({ data: [], isLoading: false, isError: false }),
}))

vi.mock('@/components/trade/LimitOrderLadderPanel', () => ({
  LimitOrderLadderPanel: () => <div data-testid="limit-order-ladder-panel" />,
}))

vi.mock('@/hooks/useTradingBlacklist', () => ({
  useTradingBlacklist: vi.fn(),
}))

import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import {
  TRADING_BLACKLIST_ALLOWED,
  pairBlacklistedResponse,
  tokenBlacklistedResponse,
  tradingBlacklistHookResult,
  walletBlacklistedResponse,
} from '@/test/tradingBlacklistMocks'
import { describeTradingBlacklistBlock } from '@/services/terraclassic/blacklist'

async function selectLimitsPair(user: ReturnType<typeof userEvent.setup>) {
  await waitFor(() => expect(factory.getAllPairsPaginated).toHaveBeenCalled())
  const pairControl = await screen.findByLabelText('Trading pair')
  await user.click(pairControl)
  const listbox = await screen.findByRole('listbox')
  const pairOption = within(listbox).getByRole('option', { name: /terra1pa.*000001/i })
  await user.click(pairOption)
  await waitFor(() => expect(indexerClient.getPair).toHaveBeenCalledWith(PAIR))
}

describe('LimitOrdersPage', () => {
  beforeEach(() => {
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    vi.mocked(getPairPaused).mockResolvedValue({ paused: false })
    vi.mocked(updateLimitOrderPrice).mockClear()
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.mocked(factory.getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: PAIR,
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [
            { token: { contract_addr: 'terra1aaa0000000000000000000000000000001' } },
            { token: { contract_addr: 'terra1bbb0000000000000000000000000000002' } },
          ],
        },
      ],
    })
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [mockIndexerPair],
      total: 1,
      limit: 20,
      offset: 0,
    })
    vi.mocked(indexerClient.getPair).mockResolvedValue(mockIndexerPair)
    vi.mocked(indexerClient.getTrades).mockResolvedValue([
      {
        trade_id: 1,
        pair_address: PAIR,
        price: '3',
        side: 'buy',
        amount_base: '1000000',
        amount_quote: '3000000',
        timestamp: '2026-01-01T00:00:00Z',
        tx_hash: 'abc',
      },
    ])
    vi.mocked(indexerClient.getPairLimitBookPage).mockResolvedValue({
      side: 'bid',
      orders: [],
      has_more: false,
      next_after_order_id: null,
    })
    vi.mocked(indexerClient.getTraderLimitPlacements).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitCancellations).mockResolvedValue([])
  })

  it('shows pre-submit summary with chain label before place (GitLab #461)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    expect(screen.getByTestId('limits-pre-submit-details')).toBeInTheDocument()
    await user.click(screen.getByText('Signing details'))

    const summary = await screen.findByTestId('limits-page-pre-submit-summary')
    expect(summary.textContent).toMatch(/Action/i)
    expect(summary.textContent).toMatch(/Pay/i)
    expect(screen.getByTestId('limits-page-pre-submit-summary-action')).toHaveTextContent('Place Limit Order')
    expect(screen.getByTestId('limits-page-pre-submit-summary-chain')).toHaveTextContent('LocalTerra')
  })

  it('styles place mode toggle with btn-primary and btn-muted (#415)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })

    const ladder = await screen.findByTestId('limit-place-mode-ladder')
    const single = screen.getByRole('button', { name: /^single$/i })
    expect(single).toHaveClass('btn-primary')
    expect(ladder).toHaveClass('btn-muted')

    await user.click(ladder)
    expect(ladder).toHaveClass('btn-primary')
    expect(single).toHaveClass('btn-muted')
  })

  it('shows ladder create panel when Ladder mode selected while disconnected (GitLab #494)', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: null, walletType: null, error: null })
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    expect(screen.queryByTestId('limit-order-ladder-panel')).not.toBeInTheDocument()
    await user.click(screen.getByTestId('limit-place-mode-ladder'))
    expect(await screen.findByTestId('limit-order-ladder-panel')).toBeInTheDocument()
  })

  it('shows limits market-data outage banner when workspace indexer queries fail (GitLab #218)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getPairLimitBookPage).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    const banner = await screen.findByTestId('limits-market-data-outage-banner')
    expect(banner.textContent).toMatch(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(banner.textContent).toMatch(/book|tape|limits may be limited/i)
    expect(banner.textContent).not.toMatch(/pool reserves/i)
    expect(await screen.findByTestId('trade-book-unavailable-bid')).toBeInTheDocument()
    expect(await screen.findByTestId('trade-book-unavailable-ask')).toBeInTheDocument()
  })

  it('shows outage banner when tape fails while pair metadata is cached (GitLab #218)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    expect(await screen.findByTestId('limits-market-data-outage-banner')).toBeInTheDocument()
  })

  it('does not show outage banner when indexer pair returns 404 (GitLab #218 / #177)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    await waitFor(() => {
      expect(indexerClient.getPair).toHaveBeenCalled()
    })
    expect(screen.queryByTestId('limits-market-data-outage-banner')).not.toBeInTheDocument()
  })

  it('blocks place when limit price cannot resolve during outage (GitLab #166 / #218)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    await screen.findByTestId('limits-market-data-outage-banner')
    const guard = await screen.findByTestId('limits-page-place-guard')
    expect(guard.textContent).toMatch(/cannot validate price/i)
  })

  it('shows pair-switch loading while indexer workspace queries fetch (GitLab #218 / #180)', async () => {
    const user = userEvent.setup()
    let resolvePair!: (value: IndexerPair) => void
    vi.mocked(indexerClient.getPair).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePair = resolve
        })
    )
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)
    expect(await screen.findByTestId('limits-pair-switch-loading')).toBeInTheDocument()
    resolvePair(mockIndexerPair)
    await waitFor(() => {
      expect(screen.queryByTestId('limits-pair-switch-loading')).not.toBeInTheDocument()
    })
  })

  it('book Edit prefills ticket with orderId and shows editing context (GitLab #312 / #294)', async () => {
    const user = userEvent.setup()
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: MAKER, walletType: 'station', error: null })
    vi.mocked(indexerClient.getPairLimitBookPage).mockImplementation(async (_pair, side) => ({
      side,
      orders:
        side === 'bid'
          ? [{ order_id: 7, owner: MAKER, side, price: '2.5', remaining: '1000000', expires_at: null }]
          : [],
      has_more: false,
      next_after_order_id: null,
    }))

    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)
    await user.click(await screen.findByTestId('trade-book-edit-bid-7'))

    expect(await screen.findByTestId('limits-page-edit-context')).toHaveTextContent(/Editing\s+#7/i)
    expect(screen.getByTestId('limit-order-price-input')).toHaveValue('2.5')
    expect(screen.getByTestId('limit-order-escrow-amount-input')).toHaveValue('1')
  })

  it('price-only amend on /limits submits UpdateLimitOrderPrice, not place (GitLab #312)', async () => {
    const user = userEvent.setup()
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: MAKER, walletType: 'station', error: null })
    vi.mocked(indexerClient.getPairLimitBookPage).mockImplementation(async (_pair, side) => ({
      side,
      orders:
        side === 'bid'
          ? [{ order_id: 7, owner: MAKER, side, price: '2.5', remaining: '1000000', expires_at: null }]
          : [],
      has_more: false,
      next_after_order_id: null,
    }))

    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)
    await user.click(await screen.findByTestId('trade-book-edit-bid-7'))

    const priceInput = await screen.findByTestId('limit-order-price-input')
    await user.clear(priceInput)
    await user.type(priceInput, '2')

    const updateBtn = await screen.findByTestId('limits-limit-update-price-submit')
    expect(updateBtn).toHaveTextContent(/Update price/i)
    await user.click(updateBtn)

    await waitFor(() => {
      expect(updateLimitOrderPrice).toHaveBeenCalledWith(MAKER, PAIR, 7, '2', expect.any(Number), null)
    })
  })

  it('blocks silent duplicate when side changes during book edit (GitLab #312)', async () => {
    const user = userEvent.setup()
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: MAKER, walletType: 'station', error: null })
    vi.mocked(indexerClient.getPairLimitBookPage).mockImplementation(async (_pair, side) => ({
      side,
      orders:
        side === 'bid'
          ? [{ order_id: 7, owner: MAKER, side, price: '2.5', remaining: '1000000', expires_at: null }]
          : [],
      has_more: false,
      next_after_order_id: null,
    }))

    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)
    await user.click(await screen.findByTestId('trade-book-edit-bid-7'))
    await user.click(await screen.findByTestId('limit-orders-side-ask'))

    const submit = await screen.findByTestId('limits-limit-submit')
    expect(submit).toBeDisabled()
    expect(await screen.findByTestId('limits-page-edit-context')).toHaveTextContent(
      /cancel first to change size, side, or expiry/i
    )
  })

  it('disables book Edit when pair is paused (GitLab #312 / L6)', async () => {
    const user = userEvent.setup()
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    vi.mocked(getPairPaused).mockResolvedValue({ paused: true })
    useWalletStore.setState({ address: MAKER, walletType: 'station', error: null })
    vi.mocked(indexerClient.getPairLimitBookPage).mockImplementation(async (_pair, side) => ({
      side,
      orders:
        side === 'bid'
          ? [{ order_id: 7, owner: MAKER, side, price: '2.5', remaining: '1000000', expires_at: null }]
          : [],
      has_more: false,
      next_after_order_id: null,
    }))

    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    expect(await screen.findByTestId('trade-book-edit-bid-7')).toBeDisabled()
    expect(await screen.findByText(/Pair paused/i)).toBeInTheDocument()
  })

  it('renders place card before order book (#488 layout)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    const placeCard = await screen.findByTestId('limits-place-card')
    const orderBook = await screen.findByTestId('limits-order-book-panel')
    expect(placeCard.compareDocumentPosition(orderBook) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  describe('trading blacklist UX (GitLab #388 / SEC-E01)', () => {
    async function renderConnectedWithPair() {
      const user = userEvent.setup()
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: MAKER, walletType: 'station', error: null })
      renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
      await selectLimitsPair(user)
      return { user }
    }

    it.each([
      ['wallet', walletBlacklistedResponse()],
      ['pair', pairBlacklistedResponse(PAIR)],
      ['token', tokenBlacklistedResponse('terra1aaa0000000000000000000000000000001')],
    ] as const)('shows %s blacklist alert and disables place limit CTA', async (_variant, resp) => {
      vi.mocked(useTradingBlacklist).mockReturnValue(tradingBlacklistHookResult(resp))
      await renderConnectedWithPair()

      const alert = await screen.findByRole('alert')
      expect(alert).toHaveTextContent(describeTradingBlacklistBlock(resp))
      expect(screen.getByTestId('limits-limit-submit')).toBeDisabled()
    })

    it.each([
      ['wallet', walletBlacklistedResponse()],
      ['pair', pairBlacklistedResponse(PAIR)],
      ['token', tokenBlacklistedResponse('terra1aaa0000000000000000000000000000001')],
    ] as const)('shows %s blacklist alert and disables cancel limit CTA', async (_variant, resp) => {
      vi.mocked(useTradingBlacklist).mockReturnValue(tradingBlacklistHookResult(resp))
      const { user } = await renderConnectedWithPair()

      const details = screen.getByText('Cancel by order ID').closest('details')
      expect(details).toBeTruthy()
      await user.click(details!.querySelector('summary')!)

      const orderInput = await screen.findByLabelText('Order ID')
      await user.type(orderInput, '42')

      const alert = await screen.findByRole('alert')
      expect(alert).toHaveTextContent(describeTradingBlacklistBlock(resp))
      expect(screen.getByRole('button', { name: /Cancel limit/i })).toBeDisabled()
    })
  })
})
