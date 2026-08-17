import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, within, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import ProtocolPage from './ProtocolPage'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FACTORY_CONTRACT_ADDRESS: 'terra1factory000000000000000000000000001',
    ROUTER_CONTRACT_ADDRESS: 'terra1router00000000000000000000000000001',
    TERRA_LCD_URL: 'http://localhost:1317',
    TERRA_RPC_URL: 'http://localhost:26657',
  }
})

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getOraclePrice: vi.fn(),
    getOracleHistory: vi.fn(),
    getHookEvents: vi.fn(),
    getOverview: vi.fn(),
  }
})

const overviewOk = {
  total_volume_24h: '999999',
  total_volume_24h_usd: '1234.5',
  total_volume_7d_usd: '5000',
  total_volume_30d_usd: '12000',
  total_trades_24h: 42,
  pair_count: 8,
  token_count: 15,
  tokens_added_30d: 3,
  pairs_added_30d: 2,
  active_pairs_24h: 5,
  unique_traders_24h: 9,
  ustc_price_usd: '0.005',
}

function mockOracle(ticker: string, price: string) {
  vi.mocked(indexerClient.getOraclePrice).mockImplementation(async (t = 'ustc') => ({
    ticker: t,
    price_usd: t === ticker ? price : price,
    sources: [{ source: 'test', price_usd: price, fetched_at: '2026-01-01T00:00:00Z' }],
  }))
  vi.mocked(indexerClient.getOracleHistory).mockImplementation(async (params) => ({
    ticker: params?.ticker ?? 'ustc',
    prices:
      (params?.ticker ?? 'ustc') === 'lunc'
        ? [{ price_usd: '0.00005', fetched_at: '2026-01-01T00:00:00Z' }]
        : (params?.ticker ?? 'ustc') === 'vfdusd'
          ? [{ price_usd: '0.87', fetched_at: '2026-01-01T00:00:00Z' }]
          : [{ price_usd: '0.005', fetched_at: '2026-01-01T00:00:00Z' }],
  }))
}

describe('ProtocolPage (GitLab #550 / #378)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue(overviewOk)
    mockOracle('ustc', '0.00512')
    vi.mocked(indexerClient.getHookEvents).mockResolvedValue([])
  })

  it('shows factory and router addresses for audit (GitLab #378)', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    expect(await screen.findByTestId('protocol-contract-addresses')).toBeInTheDocument()
    expect(screen.getByTestId('protocol-factory-address')).toHaveTextContent('terra1factory000000000000000000000000001')
    expect(screen.getByTestId('protocol-router-address')).toHaveTextContent('terra1router00000000000000000000000000001')
    expect(screen.getByText('http://localhost:1317')).toBeInTheDocument()
    expect(screen.getByText('http://localhost:26657')).toBeInTheDocument()
  })

  it('renders global stats above a single oracle card and does not headline mixed-unit volume', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const stats = await screen.findByTestId('protocol-global-stats')
    const oracle = await screen.findByTestId('protocol-oracle')
    expect(stats.compareDocumentPosition(oracle) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(within(stats).getByTestId('protocol-stat-volume-24h')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-volume-7d')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-volume-30d')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-tokens')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-tokens-added')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-pairs-added')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-active-pairs')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-trades-24h')).toBeInTheDocument()
    expect(screen.queryByText('999999')).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /Recent USTC\/USD history/i })).not.toBeInTheDocument()
    expect(screen.getAllByTestId('protocol-oracle')).toHaveLength(1)
  })

  it('defaults to USTC and loads price plus history for that ticker', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    expect(await screen.findByRole('heading', { name: /USTC \/ USD/i })).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('ustc')
    expect(indexerClient.getOracleHistory).toHaveBeenCalledWith({ ticker: 'ustc', limit: 48 })
    expect(screen.getByTestId('protocol-oracle-tab-ustc')).toHaveAttribute('aria-selected', 'true')
  })

  it('clicking LUNC refetches that ticker and updates the heading', async () => {
    const user = userEvent.setup()
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByRole('heading', { name: /USTC \/ USD/i })
    await user.click(screen.getByTestId('protocol-oracle-tab-lunc'))
    expect(await screen.findByRole('heading', { name: /LUNC \/ USD/i })).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('lunc')
    expect(indexerClient.getOracleHistory).toHaveBeenCalledWith({ ticker: 'lunc', limit: 48 })
  })

  it('clicking vFDUSD uses vfdusd queries and ~1-scale mock, not USTC', async () => {
    const user = userEvent.setup()
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByRole('heading', { name: /USTC \/ USD/i })
    await user.click(screen.getByTestId('protocol-oracle-tab-vfdusd'))
    expect(await screen.findByRole('heading', { name: /vFDUSD \/ USD/i })).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('vfdusd')
    expect(indexerClient.getOracleHistory).toHaveBeenCalledWith({ ticker: 'vfdusd', limit: 48 })
  })

  it('oracle tabs are keyboard accessible', async () => {
    const user = userEvent.setup()
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByTestId('protocol-oracle-tabs')
    const ustc = screen.getByTestId('protocol-oracle-tab-ustc')
    ustc.focus()
    await user.keyboard('{ArrowRight}')
    expect(await screen.findByRole('heading', { name: /LUNC \/ USD/i })).toBeInTheDocument()
  })

  it('empty history stays inside the oracle card', async () => {
    vi.mocked(indexerClient.getOracleHistory).mockResolvedValue({ ticker: 'ustc', prices: [] })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const oracle = await screen.findByTestId('protocol-oracle')
    expect(await within(oracle).findByTestId('protocol-oracle-history-empty')).toHaveTextContent(/No history/i)
  })

  it('opens ?ticker=lunc and rejects unknown query values', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=lunc' })
    expect(await screen.findByRole('heading', { name: /LUNC \/ USD/i })).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('lunc')

    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=btc' })
    expect(await screen.findAllByRole('heading', { name: /USTC \/ USD/i })).toBeTruthy()

    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=../ustc' })
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('ustc')
  })

  it('oracle price error keeps stats when overview succeeded', async () => {
    vi.mocked(indexerClient.getOraclePrice).mockRejectedValue(new Error('Indexer API error: 502'))
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    expect(await screen.findByTestId('protocol-global-stats')).toBeInTheDocument()
    expect(await screen.findByText(/Failed to load oracle price/i)).toBeInTheDocument()
    expect(screen.getByTestId('protocol-stat-volume-24h')).toBeInTheDocument()
  })

  it('overview 502 shows outage banner without host:port leak', async () => {
    vi.mocked(indexerClient.getOverview).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const banner = await screen.findByTestId('protocol-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1|:3001/i)
  })

  it('missing additive overview fields render em-dash, not NaN', async () => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue({
      total_volume_24h: '1',
      total_trades_24h: 0,
      pair_count: 0,
      token_count: 0,
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByTestId('protocol-global-stats')
    const vol7d = await screen.findByTestId('protocol-stat-volume-7d')
    await waitFor(() => expect(vol7d).toHaveTextContent(/—/))
    expect(screen.queryByText('NaN')).not.toBeInTheDocument()
    expect(screen.queryByText('undefined')).not.toBeInTheDocument()
  })
})
