import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import PortfolioPage from './PortfolioPage'
import * as indexerClient from '@/services/indexer/client'
import { useWalletStore } from '@/hooks/useWallet'
import type { IndexerPosition, IndexerTrader } from '@/types'

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getTrader: vi.fn(),
    getTraderTrades: vi.fn(),
    getTraderPositions: vi.fn(),
    getTraderLimitPlacements: vi.fn(),
    getPairs: vi.fn(),
    getOraclePrice: vi.fn(),
  }
})

vi.mock('@/hooks/usePortfolioLpBalances', () => ({
  usePortfolioLpBalances: () => ({
    data: { rows: [], pairsScanned: 0, capped: false },
    isLoading: false,
    isError: false,
    refetch: vi.fn(),
  }),
}))

const WALLET = 'terra1wallet0000000000000000000000000000000'

const mockTrader: IndexerTrader = {
  address: WALLET,
  total_trades: 12,
  total_volume: '1000',
  volume_24h: '0',
  volume_7d: '0',
  volume_30d: '0',
  tier_id: 1,
  tier_name: 'Bronze',
  registered: true,
  first_trade_at: '2024-01-01T00:00:00Z',
  last_trade_at: '2024-06-01T00:00:00Z',
  total_realized_pnl: '1.5',
  best_trade_pnl: '2',
  worst_trade_pnl: '-0.5',
  total_fees_paid: '0.1',
}

const mockPosition: IndexerPosition = {
  pair_address: 'terra1pair0000000000000000000000000000000',
  asset_0_symbol: 'UST1',
  asset_1_symbol: 'cUSTC',
  asset_0_decimals: 6,
  asset_1_decimals: 6,
  net_position_quote: '38290000',
  avg_entry_price: '0.00496',
  total_cost_base: '190000',
  realized_pnl: '25000000',
  trade_count: 3,
}

function renderPortfolio() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })
  const router = createMemoryRouter([{ path: '/portfolio', element: <PortfolioPage /> }], {
    initialEntries: ['/portfolio'],
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
}

describe('PortfolioPage (component)', () => {
  beforeEach(() => {
    useWalletStore.setState({ address: null, walletType: null })
    vi.mocked(indexerClient.getTraderTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getTraderPositions).mockResolvedValue([])
    vi.mocked(indexerClient.getTraderLimitPlacements).mockResolvedValue([])
    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({
      ticker: 'ustc',
      price_usd: '0.005',
      sources: [],
    })
  })

  it('shows connect prompt when wallet disconnected (GitLab #212)', () => {
    renderPortfolio()
    expect(screen.getByTestId('portfolio-connect-prompt')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /connect wallet/i })).toBeInTheDocument()
    expect(indexerClient.getTrader).not.toHaveBeenCalled()
    expect(indexerClient.getTraderPositions).not.toHaveBeenCalled()
  })

  it('shows profile empty state on 404 and still loads positions', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated' })
    vi.mocked(indexerClient.getTrader).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    vi.mocked(indexerClient.getTraderPositions).mockResolvedValue([])
    renderPortfolio()
    await waitFor(() => expect(screen.getByTestId('portfolio-profile-empty')).toBeInTheDocument())
    expect(screen.getByTestId('trader-positions-empty')).toBeInTheDocument()
    expect(indexerClient.getTraderPositions).toHaveBeenCalledWith(WALLET)
  })

  it('renders open limits section when wallet connected', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated' })
    vi.mocked(indexerClient.getTrader).mockResolvedValue(mockTrader)
    renderPortfolio()
    await waitFor(() => expect(screen.getByTestId('portfolio-open-limits-section')).toBeInTheDocument())
    expect(indexerClient.getTraderLimitPlacements).toHaveBeenCalledWith(WALLET, { limit: 100 })
  })

  it('renders positions table and summary when data exists', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated' })
    vi.mocked(indexerClient.getTrader).mockResolvedValue(mockTrader)
    vi.mocked(indexerClient.getTraderPositions).mockResolvedValue([mockPosition])
    renderPortfolio()
    await waitFor(() => expect(screen.getByText(/UST1\/cUSTC/i)).toBeInTheDocument())
    expect(screen.getByTestId('portfolio-positions-section')).toBeInTheDocument()
    expect(screen.getByText(/Bronze/i)).toBeInTheDocument()
    const tradeLink = screen.getByRole('link', { name: /UST1\/cUSTC/i })
    expect(tradeLink).toHaveAttribute('href', `/trade/${mockPosition.pair_address}`)
    expect(screen.getByTestId('trader-position-net')).toHaveTextContent(/38\.29 cUSTC/)
    expect(screen.getByTestId('trader-position-net').textContent).not.toMatch(/M/)
    expect(screen.getByTestId('trader-position-pnl')).toHaveTextContent(/UST1/)
    expect(screen.getByTestId('trader-total-volume-usd')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-summary-fees')).toHaveTextContent('—')
  })

  it('shows market-data outage banner on indexer transport failure', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated' })
    vi.mocked(indexerClient.getTrader).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getTraderPositions).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderPortfolio()
    const banner = await screen.findByTestId('portfolio-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
  })

  it('opens wallet modal from connect CTA', async () => {
    const user = userEvent.setup()
    renderPortfolio()
    await user.click(screen.getByRole('button', { name: /connect wallet/i }))
    expect(useWalletStore.getState().walletModalOpen).toBe(true)
  })
})
