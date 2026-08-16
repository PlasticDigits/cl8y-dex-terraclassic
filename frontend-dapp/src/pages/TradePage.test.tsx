import '@/test/lightweightChartsJsdomMock'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor, within, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, Outlet, RouterProvider, useLocation } from 'react-router-dom'
import TradePage from './TradePage'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'
import { TRADE_DESKTOP_LAYOUT_MEDIA_QUERY } from '@/utils/tradePageLayout'
import * as factory from '@/services/terraclassic/factory'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerPair } from '@/types'

const PAIR = 'terra1pair0000000000000000000000000000000001'
const MAKER = 'terra1maker000000000000000000000000000001'

/** Mirrors Layout.tsx keyed Outlet — pathname change remounts the route subtree (GitLab #358). */
function LayoutParityShell() {
  const location = useLocation()
  return <Outlet key={location.pathname} />
}

function renderTradeRoutes(initialEntries: string[], opts?: { layoutParity?: boolean }) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  const tradeRoutes = [
    { path: '/trade', element: <TradePage /> },
    { path: '/trade/:pairAddr', element: <TradePage /> },
  ]
  const router = createMemoryRouter(
    opts?.layoutParity ? [{ element: <LayoutParityShell />, children: tradeRoutes }] : tradeRoutes,
    { initialEntries }
  )
  render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
  return router
}

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

vi.mock('@/hooks/useTradingBlacklist', () => ({
  useTradingBlacklist: vi.fn(),
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
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import {
  TRADING_BLACKLIST_ALLOWED,
  pairBlacklistedResponse,
  tokenBlacklistedResponse,
  tradingBlacklistHookResult,
  walletBlacklistedResponse,
} from '@/test/tradingBlacklistMocks'
import { describeTradingBlacklistBlock } from '@/services/terraclassic/blacklist'
import { TRADE_TAPE_EXPANDED_KEY, TRADE_WALLET_HISTORY_EXPANDED_KEY } from '@/utils/tradeWorkspacePanels'

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
    window.localStorage.clear()
    window.sessionStorage.clear()
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
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
    vi.mocked(indexerClient.getTraderLimitPlacements).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitCancellations).mockResolvedValue([])
    vi.mocked(indexerClient.getCandles).mockResolvedValue([])
    vi.mocked(indexerClient.getPairStats).mockResolvedValue({ ...emptyStats })
    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({ ticker: 'ustc', price_usd: '0.02', sources: [] })
    vi.mocked(getPairPaused).mockResolvedValue({ paused: false })
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('shows first-visit onboarding strip until dismissed (GitLab #417)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    expect(await screen.findByTestId('trade-onboarding-strip')).toBeInTheDocument()
    await user.click(screen.getByTestId('trade-onboarding-dismiss'))
    expect(screen.queryByTestId('trade-onboarding-strip')).not.toBeInTheDocument()
  })

  it('defaults tape and wallet history to collapsed on first visit (GitLab #417)', async () => {
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: MAKER, walletType: 'station', error: null })
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    await screen.findByTestId('trade-sub-lg-workspace')
    expect(screen.getByTestId('trade-sub-lg-tape-disclosure-toggle')).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByTestId('trade-sub-lg-tape-disclosure-content')).not.toBeInTheDocument()
    expect(screen.getByTestId('trade-wallet-history-disclosure-toggle')).toHaveAttribute('aria-expanded', 'false')
    expect(window.localStorage.getItem(TRADE_TAPE_EXPANDED_KEY)).toBeNull()
    expect(window.localStorage.getItem(TRADE_WALLET_HISTORY_EXPANDED_KEY)).toBeNull()
  })

  it('desktop tape panel starts collapsed on first visit (GitLab #417)', async () => {
    mockTradeDesktopLayout(true)
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    await screen.findByTestId('trade-desktop-workspace')
    expect(screen.getByTestId('trade-desktop-tape-toggle')).toHaveTextContent('Expand')
    expect(screen.queryByRole('table', { name: /recent trades/i })).not.toBeInTheDocument()
    expect(window.localStorage.getItem(TRADE_TAPE_EXPANDED_KEY)).toBeNull()
  })

  it('persists tape disclosure expansion in localStorage (GitLab #417)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    await screen.findByTestId('trade-sub-lg-tape-disclosure-toggle')
    await user.click(screen.getByTestId('trade-sub-lg-tape-disclosure-toggle'))
    expect(screen.getByTestId('trade-sub-lg-tape-disclosure-content')).toBeInTheDocument()
    expect(window.localStorage.getItem(TRADE_TAPE_EXPANDED_KEY)).toBe('1')
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

  it('limit tab shows pre-submit summary before Place limit (GitLab #157, #461)', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    const summary = await screen.findByTestId('trade-limit-pre-submit-summary')
    expect(summary.textContent).toMatch(/Action/i)
    expect(summary.textContent).toMatch(/Pay/i)
    expect(screen.getByTestId('trade-limit-pre-submit-summary-action')).toHaveTextContent('Place Limit Order')
    expect(screen.getByTestId('trade-limit-pre-submit-summary-chain')).toHaveTextContent('LocalTerra')
    expect(summary.textContent).toMatch(/Maker fee/i)
    expect(within(summary).getByRole('link', { name: /^Docs$/i })).toBeInTheDocument()
  })

  it('keeps disconnected ticket wallet CTAs actionable', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    expect(await screen.findByTestId('trade-limit-submit')).not.toHaveAttribute('disabled')
    expect(screen.getByTestId('trade-cancel-submit')).not.toHaveAttribute('disabled')
  })

  it('keeps limit place guards in document flow above ticket footer (GitLab #500 / #527)', async () => {
    mockTradeDesktopLayout(true)
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    const footer = await screen.findByTestId('trade-ticket-submit-footer')
    const guards = screen.getByTestId('trade-limit-inline-guards')
    const scroll = screen.getByTestId('trade-order-ticket-scroll')
    const card = screen.getByTestId('trade-order-ticket-card')
    const submit = screen.getByTestId('trade-limit-submit')

    expect(scroll.className).toMatch(/trade-order-ticket-scroll/)
    expect(footer.className).toMatch(/trade-ticket-submit-footer/)
    expect(card).toContainElement(scroll)
    expect(card).toContainElement(footer)
    expect(scroll).toContainElement(guards)
    expect(scroll.contains(footer)).toBe(false)
    expect(footer.contains(guards)).toBe(false)
    expect(footer).toContainElement(submit)
    expect(scroll.contains(submit)).toBe(false)
    expect(guards.compareDocumentPosition(footer) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('docks Market money CTA in the same ticket footer (GitLab #527)', async () => {
    const user = userEvent.setup()
    mockTradeDesktopLayout(true)
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    await user.click(await screen.findByTestId('trade-order-tab-market'))
    const footer = await screen.findByTestId('trade-ticket-submit-footer')
    const marketSubmit = await screen.findByTestId('trade-market-submit')
    const scroll = screen.getByTestId('trade-order-ticket-scroll')

    expect(footer).toContainElement(marketSubmit)
    expect(scroll.contains(marketSubmit)).toBe(false)
    expect(screen.queryByTestId('trade-limit-submit')).not.toBeInTheDocument()
    expect(screen.getAllByTestId('trade-market-submit')).toHaveLength(1)
  })

  it('keeps ticket footer after invert toggle and side flip (GitLab #524 / #527)', async () => {
    const user = userEvent.setup()
    mockTradeDesktopLayout(true)
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    const footer = await screen.findByTestId('trade-ticket-submit-footer')
    expect(footer).toContainElement(screen.getByTestId('trade-limit-submit'))

    const invert = screen.queryByTestId('trade-ticket-pair-invert')
    if (invert) await user.click(invert)
    await user.click(screen.getByTestId('limit-order-side-flip'))

    expect(screen.getByTestId('trade-ticket-submit-footer')).toContainElement(screen.getByTestId('trade-limit-submit'))
    expect(screen.getAllByTestId('trade-limit-submit')).toHaveLength(1)
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
    const footer = screen.getByTestId('trade-ticket-submit-footer')
    const moneyCta = screen.queryByTestId('trade-limit-update-price-submit') ?? screen.getByTestId('trade-limit-submit')
    expect(footer).toContainElement(moneyCta)
  })

  it('shows accurate indexer outage banner when pair fetch fails (GitLab #164)', async () => {
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    const banner = await screen.findByTestId('trade-indexer-outage-banner')
    expect(banner.textContent).toMatch(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(banner.textContent).toMatch(/data may be limited|funds stay safe/i)
  })

  it('shows per-panel outage copy when tape fails while pair metadata is cached (GitLab #165)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    expect(await screen.findByTestId('trade-indexer-outage-banner')).toBeInTheDocument()
    await user.click(await screen.findByTestId('trade-sub-lg-tape-disclosure-toggle'))
    expect(await screen.findByTestId('trade-tape-unavailable')).toHaveTextContent(/recent trades unavailable/i)
  })

  it('shows outage copy on book, tape, and chart when indexer transport fails (GitLab #165)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getCandles).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getPairLimitBookPage).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    expect(await screen.findByTestId('trade-indexer-outage-banner')).toBeInTheDocument()
    await user.click(await screen.findByTestId('trade-sub-lg-tape-disclosure-toggle'))
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

    await waitFor(() => {
      expect(vi.mocked(indexerClient.getPair)).toHaveBeenCalledWith(PAIR)
    })
    resolvePair(mockIndexerPair)
    await waitFor(() => {
      expect(screen.queryByText(/Loading chart/i)).not.toBeInTheDocument()
    })
  })

  it('keeps non-default deep link after factory pairs resolve (GitLab #357)', async () => {
    const PAIR_B = 'terra1pair0000000000000000000000000000000002'
    const mockIndexerPairB: IndexerPair = {
      ...mockIndexerPair,
      pair_address: PAIR_B,
      asset_0: { symbol: 'CCC', contract_addr: 'terra1ccc0000000000000000000000000000003', denom: null, decimals: 6 },
      asset_1: { symbol: 'DDD', contract_addr: 'terra1ddd0000000000000000000000000000004', denom: null, decimals: 6 },
    }

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
        {
          contract_addr: PAIR_B,
          liquidity_token: 'terra1lp000000000000000000000000000000002',
          asset_infos: [
            { token: { contract_addr: 'terra1ccc0000000000000000000000000000003' } },
            { token: { contract_addr: 'terra1ddd0000000000000000000000000000004' } },
          ],
        },
      ],
    })
    vi.mocked(indexerClient.getPair).mockImplementation(async (addr) =>
      addr === PAIR_B ? mockIndexerPairB : mockIndexerPair
    )

    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const router = createMemoryRouter(
      [
        { path: '/trade', element: <TradePage /> },
        { path: '/trade/:pairAddr', element: <TradePage /> },
      ],
      { initialEntries: [`/trade/${PAIR_B}`] }
    )
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )

    await screen.findByTestId('trade-sub-lg-workspace')
    await waitFor(() => {
      expect(router.state.location.pathname).toBe(`/trade/${PAIR_B}`)
    })
    expect(vi.mocked(indexerClient.getPair)).toHaveBeenCalledWith(PAIR_B)
    const pairInput = screen.getByRole('combobox', { name: 'Trading pair' }) as HTMLInputElement
    expect(pairInput.value).toContain('000002')
  })

  it('auto-picks first factory pair on bare /trade (GitLab #357)', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const router = createMemoryRouter(
      [
        { path: '/trade', element: <TradePage /> },
        { path: '/trade/:pairAddr', element: <TradePage /> },
      ],
      { initialEntries: ['/trade'] }
    )
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )

    await waitFor(() => {
      expect(router.state.location.pathname).toBe(`/trade/${PAIR}`)
    })
    await screen.findByTestId('trade-sub-lg-workspace')
  })

  it('switches trade workspace when a different pair is selected from search (GitLab #301)', async () => {
    const PAIR_B = 'terra1pair0000000000000000000000000000000002'
    const mockIndexerPairB: IndexerPair = {
      ...mockIndexerPair,
      pair_address: PAIR_B,
      asset_0: { symbol: 'CCC', contract_addr: 'terra1ccc0000000000000000000000000000003', denom: null, decimals: 6 },
      asset_1: { symbol: 'DDD', contract_addr: 'terra1ddd0000000000000000000000000000004', denom: null, decimals: 6 },
    }

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
        {
          contract_addr: PAIR_B,
          liquidity_token: 'terra1lp000000000000000000000000000000002',
          asset_infos: [
            { token: { contract_addr: 'terra1ccc0000000000000000000000000000003' } },
            { token: { contract_addr: 'terra1ddd0000000000000000000000000000004' } },
          ],
        },
      ],
    })
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [mockIndexerPair, mockIndexerPairB],
      total: 2,
      limit: 20,
      offset: 0,
    })
    vi.mocked(indexerClient.getPair).mockImplementation(async (addr) =>
      addr === PAIR_B ? mockIndexerPairB : mockIndexerPair
    )
    vi.mocked(indexerClient.getPairLimitBookPage).mockImplementation(async (_pair, side) => ({
      side,
      orders: [],
      has_more: false,
      next_after_order_id: null,
    }))

    const user = userEvent.setup()
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const router = createMemoryRouter(
      [
        { path: '/trade', element: <TradePage /> },
        { path: '/trade/:pairAddr', element: <TradePage /> },
      ],
      { initialEntries: [`/trade/${PAIR}`] }
    )
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    )

    await screen.findByTestId('trade-sub-lg-workspace')
    const pairInput = screen.getByRole('combobox', { name: 'Trading pair' })
    expect(pairInput).toHaveAttribute('type', 'text')

    await user.click(pairInput)
    const listbox = await screen.findByRole('listbox')
    await user.click(within(listbox).getByRole('option', { name: /CCC/i }))

    await waitFor(() => {
      expect(router.state.location.pathname).toBe(`/trade/${PAIR_B}`)
      expect(vi.mocked(indexerClient.getPair)).toHaveBeenCalledWith(PAIR_B)
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

  it('keeps pair-not-found notice after Layout keyed-Outlet remount (GitLab #358)', async () => {
    const unknownPair = 'terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'
    vi.mocked(indexerClient.getPair).mockClear()
    vi.mocked(indexerClient.getTrades).mockClear()
    const router = renderTradeRoutes([`/trade/${unknownPair}`], { layoutParity: true })

    const notice = await screen.findByTestId('trade-pair-not-found-link-notice')
    expect(notice.textContent).toMatch(/pair not found/i)
    expect(screen.getByTestId('trade-pair-not-found-link-value').textContent).toContain(unknownPair.slice(0, 12))

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/trade')
    })
    expect(router.state.location.pathname).not.toBe(`/trade/${PAIR}`)
    expect(screen.getByTestId('trade-pair-not-found-link-notice')).toBeInTheDocument()

    const pairSelect = await screen.findByLabelText('Trading pair')
    expect((pairSelect as HTMLInputElement).value).not.toContain(unknownPair)
    expect(indexerClient.getPair).not.toHaveBeenCalled()
    expect(indexerClient.getTrades).not.toHaveBeenCalled()
    expect(screen.queryByTestId('trade-sub-lg-workspace')).not.toBeInTheDocument()
  })

  it('keeps invalid pair link notice after Layout keyed-Outlet remount (GitLab #358)', async () => {
    vi.mocked(indexerClient.getPair).mockClear()
    vi.mocked(indexerClient.getTrades).mockClear()
    const router = renderTradeRoutes(['/trade/lilwayne%20babyyy'], { layoutParity: true })

    const notice = await screen.findByTestId('trade-invalid-pair-link-notice')
    expect(notice.textContent).toMatch(/invalid pair link/i)

    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/trade')
    })
    expect(router.state.location.pathname).not.toBe(`/trade/${PAIR}`)
    expect(screen.getByTestId('trade-invalid-pair-link-notice')).toBeInTheDocument()

    const pairSelect = await screen.findByLabelText('Trading pair')
    expect((pairSelect as HTMLInputElement).value).not.toMatch(/lilwayne/i)
    expect(indexerClient.getPair).not.toHaveBeenCalled()
    expect(indexerClient.getTrades).not.toHaveBeenCalled()
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

    const banner = await screen.findByText(/Pair paused/i)
    expect(banner).toBeInTheDocument()

    const placeBtns = screen.getAllByTestId('trade-limit-submit')
    expect(placeBtns.length).toBeGreaterThan(0)
    for (const btn of placeBtns) {
      expect(btn).toBeDisabled()
    }
  })

  describe('trading blacklist UX on TradeOrderTicket (GitLab #388 / SEC-A02)', () => {
    it.each([
      ['wallet', walletBlacklistedResponse()],
      ['pair', pairBlacklistedResponse(PAIR)],
      ['token', tokenBlacklistedResponse('terra1aaa0000000000000000000000000000001')],
    ] as const)('shows %s blacklist alert copy and disables limit Place CTA', async (_variant, resp) => {
      vi.mocked(useTradingBlacklist).mockReturnValue(tradingBlacklistHookResult(resp))
      renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

      const alert = await screen.findByRole('alert')
      expect(alert).toHaveTextContent(describeTradingBlacklistBlock(resp))

      const placeBtns = screen.getAllByTestId('trade-limit-submit')
      expect(placeBtns.length).toBeGreaterThan(0)
      for (const btn of placeBtns) {
        expect(btn).toBeDisabled()
      }
    })
  })

  it('clears pair-not-found notice when selecting a valid factory pair', async () => {
    const unknownPair = 'terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'
    const user = userEvent.setup()
    const router = renderTradeRoutes([`/trade/${unknownPair}`])

    await screen.findByTestId('trade-pair-not-found-link-notice')
    await waitFor(() => {
      expect(router.state.location.pathname).toBe('/trade')
    })
    expect(screen.queryByTestId('trade-sub-lg-workspace')).not.toBeInTheDocument()

    const pairSelect = await screen.findByLabelText('Trading pair')
    await user.click(pairSelect)
    const listbox = await screen.findByRole('listbox')
    await user.click(within(listbox).getByRole('option', { name: /terra1aaa/i }))

    await waitFor(() => {
      expect(screen.queryByTestId('trade-pair-not-found-link-notice')).not.toBeInTheDocument()
    })
    expect(router.state.location.pathname).toBe(`/trade/${PAIR}`)
    expect(router.state.location.state).toBeNull()
    expect(await screen.findByTestId('trade-sub-lg-workspace')).toBeInTheDocument()
  })

  it('clears stale link notice when landing on a known factory deep link', async () => {
    const unknownPair = 'terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'
    const router = renderTradeRoutes([{ pathname: '/trade', state: { unknownPair } }])

    await screen.findByTestId('trade-pair-not-found-link-notice')
    expect(screen.queryByTestId('trade-sub-lg-workspace')).not.toBeInTheDocument()

    await act(async () => {
      await router.navigate(`/trade/${PAIR}`)
    })

    await waitFor(() => {
      expect(screen.queryByTestId('trade-pair-not-found-link-notice')).not.toBeInTheDocument()
    })
    expect(router.state.location.pathname).toBe(`/trade/${PAIR}`)
    expect(router.state.location.state).toBeNull()
    expect(await screen.findByTestId('trade-sub-lg-workspace')).toBeInTheDocument()
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

  describe('UST1 pair display invert (GitLab #524)', () => {
    const UST1_PAIR = 'terra1ust1pair000000000000000000000000000001'
    const OTHER_PAIR = PAIR
    const ust1IndexerPair: IndexerPair = {
      ...mockIndexerPair,
      pair_address: UST1_PAIR,
      asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1tok000000000000000000000000001', denom: null, decimals: 6 },
      asset_1: { symbol: 'cUSTC', contract_addr: 'terra1custctok0000000000000000000000002', denom: null, decimals: 6 },
    }

    function mockUst1AndOtherPairs() {
      vi.mocked(factory.getAllPairsPaginated).mockResolvedValue({
        pairs: [
          {
            contract_addr: UST1_PAIR,
            liquidity_token: 'terra1lpust1000000000000000000000000001',
            asset_infos: [
              { token: { contract_addr: 'terra1ust1tok000000000000000000000000001' } },
              { token: { contract_addr: 'terra1custctok0000000000000000000000002' } },
            ],
          },
          {
            contract_addr: OTHER_PAIR,
            liquidity_token: 'terra1lp000000000000000000000000000000001',
            asset_infos: [
              { token: { contract_addr: 'terra1aaa0000000000000000000000000000001' } },
              { token: { contract_addr: 'terra1bbb0000000000000000000000000000002' } },
            ],
          },
        ],
      })
      vi.mocked(indexerClient.getPair).mockImplementation(async (addr) =>
        addr === UST1_PAIR ? ust1IndexerPair : mockIndexerPair
      )
      vi.mocked(indexerClient.getTrades).mockResolvedValue([
        {
          id: 1,
          pair_address: UST1_PAIR,
          block_height: 1,
          block_timestamp: '2026-08-15T00:00:00Z',
          tx_hash: 'AA',
          sender: 'terra1t',
          offer_asset: 'UST1',
          ask_asset: 'cUSTC',
          offer_amount: '1000000',
          return_amount: '206000000',
          price: '206',
          price_usd: '1',
        },
      ])
    }

    it('defaults UST1/cUSTC to other-side pill, heading, and no Order ticket', async () => {
      mockUst1AndOtherPairs()
      renderWithProviders(<TradePage />, { route: `/trade/${UST1_PAIR}` })
      await waitFor(() => {
        expect(screen.getByTestId('trade-pair-invert-pill')).toHaveTextContent('cUSTC/UST1')
      })
      expect(await screen.findByTestId('trade-ticket-heading')).toHaveTextContent('Buy cUSTC')
      expect(screen.getByTestId('trade-ticket-pair-invert')).toBeInTheDocument()
      expect(screen.queryByText(/order ticket/i)).not.toBeInTheDocument()
      const last = await screen.findByTestId('trade-chart-headline-price')
      expect(last.textContent ?? '').not.toMatch(/^\s*Last\s*1(\.0+)?\s*$/)
    })

    it('pill toggle reciprocates typed limit price so factory submit stays token1/token0 (H1)', async () => {
      mockUst1AndOtherPairs()
      const user = userEvent.setup()
      renderWithProviders(<TradePage />, { route: `/trade/${UST1_PAIR}` })
      await waitFor(() => {
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy cUSTC')
      })
      const priceInput = screen.getByTestId('limit-order-price-input')
      await user.clear(priceInput)
      await user.type(priceInput, '0.00485')
      await user.click(screen.getByTestId('trade-pair-invert-pill'))
      await waitFor(() => {
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy UST1')
      })
      const afterPill = (screen.getByTestId('limit-order-price-input') as HTMLInputElement).value
      expect(parseFloat(afterPill)).toBeCloseTo(1 / 0.00485, 5)
      await user.click(screen.getByTestId('trade-ticket-pair-invert'))
      await waitFor(() => {
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy cUSTC')
      })
      const afterIcon = (screen.getByTestId('limit-order-price-input') as HTMLInputElement).value
      expect(parseFloat(afterIcon)).toBeCloseTo(0.00485, 5)
    })

    it('pill and ticket icon share one invert state', async () => {
      mockUst1AndOtherPairs()
      const user = userEvent.setup()
      renderWithProviders(<TradePage />, { route: `/trade/${UST1_PAIR}` })
      await waitFor(() => {
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy cUSTC')
      })
      await user.click(screen.getByTestId('trade-pair-invert-pill'))
      await waitFor(() => {
        expect(screen.getByTestId('trade-pair-invert-pill')).toHaveTextContent('UST1/cUSTC')
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy UST1')
      })
      await user.click(screen.getByTestId('trade-ticket-pair-invert'))
      await waitFor(() => {
        expect(screen.getByTestId('trade-pair-invert-pill')).toHaveTextContent('cUSTC/UST1')
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy cUSTC')
      })
    })

    it('does not invert when switching to a non-UST1 pair (H4)', async () => {
      mockUst1AndOtherPairs()
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [ust1IndexerPair, mockIndexerPair],
        total: 2,
        limit: 20,
        offset: 0,
      })
      const user = userEvent.setup()
      renderWithProviders(<TradePage />, { route: `/trade/${UST1_PAIR}` })
      await waitFor(() => {
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy cUSTC')
      })
      const pairInput = screen.getByRole('combobox', { name: 'Trading pair' })
      await user.click(pairInput)
      const listbox = await screen.findByRole('listbox')
      await user.click(within(listbox).getByRole('option', { name: /AAA/i }))
      await waitFor(() => {
        expect(screen.getByTestId('trade-ticket-heading')).toHaveTextContent('Buy AAA')
      })
      expect(screen.getByTestId('trade-pair-invert-pill')).toHaveTextContent('AAA/BBB')
    })
  })
})
