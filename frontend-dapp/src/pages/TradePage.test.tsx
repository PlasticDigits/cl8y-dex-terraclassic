import '@/test/lightweightChartsJsdomMock'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import TradePage from './TradePage'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'
import { TRADE_DESKTOP_LAYOUT_MEDIA_QUERY } from '@/utils/tradePageLayout'
import * as factory from '@/services/terraclassic/factory'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerPair } from '@/types'

const PAIR = 'terra1pair0000000000000000000000000000000001'
const MAKER = 'terra1maker000000000000000000000000000001'

function mockTradeDesktopLayout(matchesDesktop: boolean) {
  window.matchMedia = vi.fn((query: string) => {
    const mql = {
      matches: query === TRADE_DESKTOP_LAYOUT_MEDIA_QUERY ? matchesDesktop : false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: (_type: string, cb: (e: MediaQueryListEvent) => void) => {
        if (query === TRADE_DESKTOP_LAYOUT_MEDIA_QUERY) cb({ matches: matchesDesktop } as MediaQueryListEvent)
      },
      removeEventListener: () => {},
      dispatchEvent: () => false,
    } as unknown as MediaQueryList
    return mql
  })
}

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
  sounds: { playButtonPress: vi.fn() },
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
    getPairLimitCancellations: vi.fn(),
    getCandles: vi.fn(),
    getPairStats: vi.fn(),
    getOraclePrice: vi.fn(),
  }
})

vi.mock('@/services/terraclassic/pair', () => ({
  getPairPaused: vi.fn().mockResolvedValue({ paused: false }),
  placeLimitOrder: vi.fn(),
  cancelLimitOrder: vi.fn(),
}))

import { getPairPaused } from '@/services/terraclassic/pair'

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({ fee_bps: 30, treasury: 'terra1treasury0000000000000000000001' }),
}))

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn(),
}))

import { getConnectedWallet } from '@/services/terraclassic/wallet'

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

const emptyStats = {
  volume_base: '0',
  volume_quote: '0',
  trade_count: 0,
  high: null,
  low: null,
  open_price: null,
  close_price: null,
  price_change_pct: null,
} as const

describe('TradePage', () => {
  beforeEach(() => {
    mockTradeDesktopLayout(false)
    vi.mocked(getConnectedWallet).mockReturnValue(null)
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
    vi.mocked(indexerClient.getTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitBookPage).mockResolvedValue({
      side: 'bid',
      orders: [],
      has_more: false,
      next_after_order_id: null,
    })
    vi.mocked(indexerClient.getPairLimitPlacements).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitCancellations).mockResolvedValue([])
    vi.mocked(indexerClient.getCandles).mockResolvedValue([])
    vi.mocked(indexerClient.getPairStats).mockResolvedValue({ ...emptyStats })
    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({ price_usd: '0.02', sources: [] })
    vi.mocked(getPairPaused).mockResolvedValue({ paused: false })
  })

  it('sub-desktop workspace uses md two-column grid for tablet portrait (GitLab #146)', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    const workspace = await screen.findByTestId('trade-sub-lg-workspace')
    expect(workspace.className).toContain('md:grid-cols-2')
    expect(workspace.className).toContain('lg:hidden')
  })

  it('order ticket exposes Limit and Market tabs (GitLab #152)', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    expect(await screen.findByTestId('trade-order-tab-market')).toBeInTheDocument()
    expect(screen.getByTestId('trade-order-tab-limit')).toBeInTheDocument()
  })

  it('limit tab shows pre-submit summary before Place limit (GitLab #157)', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    const summary = await screen.findByTestId('trade-limit-pre-submit-summary')
    expect(summary.textContent).toMatch(/no taker slippage/i)
    expect(summary.textContent).toMatch(/Maker placement fee/i)
  })

  it('keeps disconnected ticket wallet CTAs actionable', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    expect(await screen.findByTestId('trade-limit-submit')).not.toHaveAttribute('disabled')
    expect(screen.getByTestId('trade-cancel-submit')).not.toHaveAttribute('disabled')
  })

  it('book Edit prefills the visible desktop limit ticket (GitLab #178)', async () => {
    const user = userEvent.setup()
    mockTradeDesktopLayout(true)
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: MAKER, walletType: 'station', error: null })

    vi.mocked(indexerClient.getPairLimitBookPage).mockImplementation(async (_pair, side) => ({
      side,
      orders: side === 'bid' ? [{ order_id: 7, owner: MAKER, side, price: '2.5', remaining: '1000000' }] : [],
      has_more: false,
      next_after_order_id: null,
    }))

    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    await screen.findByTestId('trade-desktop-workspace')
    await user.click(await screen.findByTestId('trade-book-edit-bid-7'))

    const priceInput = await screen.findByTestId('limit-order-price-input')
    expect(priceInput).toHaveValue('2.5')
    expect(screen.getByTestId('trade-order-tab-limit')).toHaveAttribute('aria-selected', 'true')
  })

  it('shows accurate indexer outage banner when pair fetch fails (GitLab #164)', async () => {
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    const banner = await screen.findByTestId('trade-indexer-outage-banner')
    expect(banner.textContent).toMatch(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/still use chain|VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(banner.textContent).toMatch(/order book|chart|tape/i)
  })

  it('shows per-panel outage copy when tape fails while pair metadata is cached (GitLab #165)', async () => {
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    expect(await screen.findByTestId('trade-indexer-outage-banner')).toBeInTheDocument()
    expect(await screen.findByTestId('trade-tape-unavailable')).toHaveTextContent(/recent trades are unavailable/i)
  })

  it('shows outage copy on book, tape, and chart when indexer transport fails (GitLab #165)', async () => {
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getCandles).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getPairLimitBookPage).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    expect(await screen.findByTestId('trade-indexer-outage-banner')).toBeInTheDocument()
    expect(await screen.findByTestId('trade-tape-unavailable')).toBeInTheDocument()
    expect(await screen.findByTestId('trade-chart-unavailable')).toBeInTheDocument()
    expect(await screen.findByTestId('trade-book-unavailable-bid')).toBeInTheDocument()
    expect(await screen.findByTestId('trade-book-unavailable-ask')).toBeInTheDocument()
  })

  it('shows invalid pair link notice and clears garbage URL for non-terra1 deep links (GitLab #176)', async () => {
    vi.mocked(indexerClient.getPair).mockClear()
    vi.mocked(indexerClient.getTrades).mockClear()
    const user = userEvent.setup()
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const router = createMemoryRouter(
      [
        { path: '/trade', element: <TradePage /> },
        { path: '/trade/:pairAddr', element: <TradePage /> },
      ],
      { initialEntries: ['/trade/lilwayne%20babyyy'] }
    )
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )

    const notice = await screen.findByTestId('trade-invalid-pair-link-notice')
    expect(notice.textContent).toMatch(/invalid pair link/i)
    expect(screen.getByTestId('trade-invalid-pair-link-value').textContent).toMatch(/lilwayne babyyy/i)

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/trade')
    })

    const pairSelect = await screen.findByLabelText('Trading pair')
    expect((pairSelect as HTMLInputElement).value).not.toMatch(/lilwayne/i)

    expect(indexerClient.getPair).not.toHaveBeenCalled()
    expect(indexerClient.getTrades).not.toHaveBeenCalled()

    await user.click(screen.getByTestId('trade-invalid-pair-link-cta'))
    await waitFor(() => {
      expect(pairSelect).toHaveFocus()
    })
  })

  it('mounts chart and fetches candles before getPair resolves (GitLab #180)', async () => {
    let resolvePair!: (value: IndexerPair) => void
    vi.mocked(indexerClient.getPair).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePair = resolve
        })
    )
    vi.mocked(indexerClient.getCandles).mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve([]), 500))
    )

    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    await waitFor(() => {
      expect(vi.mocked(indexerClient.getCandles)).toHaveBeenCalledWith(PAIR, '1h')
    })
    expect(await screen.findByText(/Loading chart/i)).toBeInTheDocument()

    resolvePair(mockIndexerPair)
    await waitFor(() => {
      expect(screen.queryByText(/Loading chart/i)).not.toBeInTheDocument()
    })
  })

  it('shows pair switch loading status while workspace queries are in flight (GitLab #180)', async () => {
    vi.mocked(indexerClient.getCandles).mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve([]), 300))
    )
    vi.mocked(indexerClient.getTrades).mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve([]), 300))
    )

    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    expect(await screen.findByTestId('trade-pair-switch-loading')).toBeInTheDocument()
    await waitFor(() => {
      expect(screen.queryByTestId('trade-pair-switch-loading')).not.toBeInTheDocument()
    })
  })

  it('shows pair-not-found notice for valid-format deep links absent from factory (GitLab #175)', async () => {
    const unknownPair = 'terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'
    vi.mocked(indexerClient.getPair).mockClear()
    vi.mocked(indexerClient.getTrades).mockClear()
    const user = userEvent.setup()
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const router = createMemoryRouter(
      [
        { path: '/trade', element: <TradePage /> },
        { path: '/trade/:pairAddr', element: <TradePage /> },
      ],
      { initialEntries: [`/trade/${unknownPair}`] }
    )
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )

    const notice = await screen.findByTestId('trade-pair-not-found-link-notice')
    expect(notice.textContent).toMatch(/pair not found/i)
    expect(screen.getByTestId('trade-pair-not-found-link-value').textContent).toContain(unknownPair.slice(0, 12))

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/trade')
    })

    const pairSelect = await screen.findByLabelText('Trading pair')
    expect((pairSelect as HTMLInputElement).value).not.toContain(unknownPair)

    expect(indexerClient.getPair).not.toHaveBeenCalled()
    expect(indexerClient.getTrades).not.toHaveBeenCalled()

    await user.click(screen.getByTestId('trade-pair-not-found-link-cta'))
    await waitFor(() => {
      expect(pairSelect).toHaveFocus()
    })

    expect(screen.queryByTestId('trade-sub-lg-workspace')).not.toBeInTheDocument()
    expect(screen.queryByTestId('trade-desktop-workspace')).not.toBeInTheDocument()
  })

  it('does not render empty trade workspace while unknown deep link resolves (GitLab #175)', async () => {
    const unknownPair = 'terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'
    let resolveFactory!: (value: Awaited<ReturnType<typeof factory.getAllPairsPaginated>>) => void
    vi.mocked(factory.getAllPairsPaginated).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveFactory = resolve
        })
    )
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const router = createMemoryRouter(
      [
        { path: '/trade', element: <TradePage /> },
        { path: '/trade/:pairAddr', element: <TradePage /> },
      ],
      { initialEntries: [`/trade/${unknownPair}`] }
    )
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )

    expect(screen.queryByTestId('trade-sub-lg-workspace')).not.toBeInTheDocument()
    expect(screen.queryByTestId('trade-pair-not-found-link-notice')).not.toBeInTheDocument()

    resolveFactory({
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

    await screen.findByTestId('trade-pair-not-found-link-notice')
    expect(screen.queryByTestId('trade-sub-lg-workspace')).not.toBeInTheDocument()
  })

  it('keeps trade workspace for known factory pair when indexer pair 404s (GitLab #177)', async () => {
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    await waitFor(() => {
      expect(screen.getByTestId('trade-sub-lg-workspace')).toBeInTheDocument()
    })
    expect(screen.queryByTestId('trade-pair-not-found-link-notice')).not.toBeInTheDocument()
  })

  it('rejects charset-invalid terra1 deep links without indexer calls (GitLab #176 / #175)', async () => {
    const invalidPair = "terra1damThat'scrazy"
    vi.mocked(indexerClient.getPair).mockClear()
    vi.mocked(indexerClient.getTrades).mockClear()
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const router = createMemoryRouter(
      [
        { path: '/trade', element: <TradePage /> },
        { path: '/trade/:pairAddr', element: <TradePage /> },
      ],
      { initialEntries: [`/trade/${encodeURIComponent(invalidPair)}`] }
    )
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )

    await screen.findByTestId('trade-invalid-pair-link-notice')
    expect(screen.queryByTestId('trade-pair-not-found-link-notice')).not.toBeInTheDocument()
    expect(indexerClient.getPair).not.toHaveBeenCalled()
    expect(indexerClient.getTrades).not.toHaveBeenCalled()
  })

  it('shows pause banner and disables limit actions when pair is paused (GitLab #87 / #199)', async () => {
    vi.mocked(getPairPaused).mockResolvedValue({ paused: true })
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    const banner = await screen.findByText(/Pair is paused — swaps, limit place/i)
    expect(banner).toHaveTextContent(/governance unpauses/i)

    const placeBtns = screen.getAllByTestId('trade-limit-submit')
    expect(placeBtns.length).toBeGreaterThan(0)
    for (const btn of placeBtns) {
      expect(btn).toBeDisabled()
    }
  })

  it('chart Retry refetches indexer pair after 404 (GitLab #177)', async () => {
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    const user = userEvent.setup()
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    await waitFor(() => {
      expect(screen.getAllByTestId('trade-chart-retry-error').length).toBeGreaterThan(0)
    })
    const retryBtn = screen.getAllByTestId('retry-error-button')[0]
    const callsAfterError = vi.mocked(indexerClient.getPair).mock.calls.length
    expect(callsAfterError).toBeGreaterThanOrEqual(1)

    await user.click(retryBtn)

    await waitFor(() => {
      expect(vi.mocked(indexerClient.getPair).mock.calls.length).toBeGreaterThan(callsAfterError)
    })
  })
})
