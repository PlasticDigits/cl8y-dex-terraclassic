import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, within, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import ProtocolPage from './ProtocolPage'
import * as indexerClient from '@/services/indexer/client'
import {
  PROTOCOL_TRADES_24H_LABEL,
  PROTOCOL_VOLUME_24H_LABEL,
  PROTOCOL_VOLUME_7D_LABEL,
  PROTOCOL_VOLUME_30D_LABEL,
  TRAILING_24H_VOLUME_TITLE,
  TRAILING_7D_VOLUME_TITLE,
  TRAILING_30D_VOLUME_TITLE,
} from '@/utils/trailingWindowCopy'

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
    getHubPrices: vi.fn(),
  }
})

const hubPricesOk = {
  metadata: 'DEX hub USD marks. Not CEX.',
  tickers: ['custc', 'ust1', 'ustr'],
  prices: [
    { ticker: 'custc', price_usd: '0.005', source_pair: null, tvl_usd: null, updated_at: '2026-01-01T00:00:00Z' },
    {
      ticker: 'ust1',
      price_usd: '0.98',
      source_pair: 'terra1ust1custcpair',
      tvl_usd: '500',
      updated_at: '2026-01-01T00:00:00Z',
    },
    {
      ticker: 'ustr',
      price_usd: '0.012',
      source_pair: 'terra1ustrust1pair',
      tvl_usd: '200',
      updated_at: '2026-01-01T00:00:00Z',
    },
  ],
}

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
  total_liquidity_usd: '8900.25',
  liquidity_change_24h_pct: '-3.5',
  liquidity_change_30d_pct: '12.5',
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

describe('ProtocolPage (GitLab #550 / #378 / #569)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue(overviewOk)
    vi.mocked(indexerClient.getHubPrices).mockResolvedValue(hubPricesOk)
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
    const hub = await screen.findByTestId('protocol-dex-hub-prices')
    const oracle = await screen.findByTestId('protocol-oracle')
    expect(stats.compareDocumentPosition(hub) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(hub.compareDocumentPosition(oracle) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(within(stats).getByTestId('protocol-stat-liquidity')).toHaveTextContent('$')
    expect(within(stats).getByTestId('protocol-stat-liquidity-24h')).toHaveTextContent('%')
    expect(within(stats).getByTestId('protocol-stat-liquidity-24h')).toHaveTextContent('-')
    expect(within(stats).getByTestId('protocol-stat-liquidity-30d')).toHaveTextContent('+')
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

  it('discloses trailing 24h/7d/30d volume without a lecture banner (GitLab #576)', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const stats = await screen.findByTestId('protocol-global-stats')
    const vol24 = within(stats).getByTestId('protocol-stat-volume-24h')
    const vol7d = within(stats).getByTestId('protocol-stat-volume-7d')
    const vol30d = within(stats).getByTestId('protocol-stat-volume-30d')
    const trades = within(stats).getByTestId('protocol-stat-trades-24h')
    expect(vol24).toHaveTextContent(PROTOCOL_VOLUME_24H_LABEL)
    expect(vol7d).toHaveTextContent(PROTOCOL_VOLUME_7D_LABEL)
    expect(vol30d).toHaveTextContent(PROTOCOL_VOLUME_30D_LABEL)
    expect(trades).toHaveTextContent(PROTOCOL_TRADES_24H_LABEL)
    await waitFor(() => {
      expect(within(vol24).getByLabelText(/last 24 hours, not a midnight reset/i)).toBeInTheDocument()
    })
    expect(within(vol7d).getByLabelText(/last 7 days, not a calendar-week reset/i)).toBeInTheDocument()
    expect(within(vol30d).getByLabelText(/last 30 days, not a calendar-month reset/i)).toBeInTheDocument()
    expect(within(vol24).getByText(PROTOCOL_VOLUME_24H_LABEL)).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
    expect(within(vol7d).getByText(PROTOCOL_VOLUME_7D_LABEL)).toHaveAttribute('title', TRAILING_7D_VOLUME_TITLE)
    expect(within(vol30d).getByText(PROTOCOL_VOLUME_30D_LABEL)).toHaveAttribute('title', TRAILING_30D_VOLUME_TITLE)
    expect(stats).toHaveTextContent(/USD volume and pool TVL use the same USTC \/ LUNC \/ hub reference catalog/i)
    expect(stats.textContent).not.toMatch(/resets at 00:00|calendar-day volume|always-on/i)
    expect(stats.textContent).not.toMatch(/VITE_INDEXER_URL|https?:\/\//i)
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
    expect(screen.getByTestId('protocol-stat-liquidity')).toHaveTextContent(/—/)
    expect(screen.getByTestId('protocol-stat-liquidity-24h')).toHaveTextContent(/—/)
    expect(screen.getByTestId('protocol-stat-liquidity-30d')).toHaveTextContent(/—/)
  })

  it('idle TVL $0 with null Δ% is not 0% or Infinity', async () => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue({
      total_volume_24h: '1',
      total_trades_24h: 0,
      pair_count: 0,
      token_count: 0,
      total_liquidity_usd: '0',
      liquidity_change_24h_pct: null,
      liquidity_change_30d_pct: null,
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const liq = await screen.findByTestId('protocol-stat-liquidity')
    await waitFor(() => expect(liq).toHaveTextContent(/\$0/))
    expect(screen.getByTestId('protocol-stat-liquidity-24h')).toHaveTextContent(/—/)
    expect(screen.getByTestId('protocol-stat-liquidity-30d')).toHaveTextContent(/—/)
    expect(screen.queryByText('Infinity')).not.toBeInTheDocument()
    expect(screen.queryByText('0%')).not.toBeInTheDocument()
  })

  it('renders DEX hub card for cUSTC / UST1 / USTR and never queries CEX ustr', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const hub = await screen.findByTestId('protocol-dex-hub-prices')
    expect(await within(hub).findByTestId('protocol-dex-hub-custc-usd')).toHaveTextContent('$')
    expect(within(hub).getByTestId('protocol-dex-hub-ust1-usd')).toHaveTextContent('$')
    expect(within(hub).getByTestId('protocol-dex-hub-ustr-usd')).toHaveTextContent('$')
    expect(hub).toHaveTextContent(/DEX reference — not CEX, not settlement/i)
    expect(indexerClient.getHubPrices).toHaveBeenCalled()
    expect(indexerClient.getOraclePrice).not.toHaveBeenCalledWith('ustr')
    expect(indexerClient.getOraclePrice).not.toHaveBeenCalledWith('ust1')
    expect(indexerClient.getOraclePrice).not.toHaveBeenCalledWith('custc')
    expect(screen.getByTestId('protocol-oracle-tabs').querySelectorAll('[role="tab"]')).toHaveLength(3)
  })

  it('null hub prices render em-dash, not $0 or $1', async () => {
    vi.mocked(indexerClient.getHubPrices).mockResolvedValue({
      metadata: 'DEX hub USD marks. Not CEX.',
      tickers: ['custc', 'ust1', 'ustr'],
      prices: [
        { ticker: 'custc', price_usd: null },
        { ticker: 'ust1', price_usd: null },
        { ticker: 'ustr', price_usd: null },
      ],
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    expect(await screen.findByTestId('protocol-dex-hub-ustr-usd')).toHaveTextContent('—')
    expect(screen.getByTestId('protocol-dex-hub-ust1-usd')).toHaveTextContent('—')
    expect(screen.queryByTestId('protocol-dex-hub-ustr-usd')?.textContent).not.toMatch(/\$0|\$1|2\.5/)
  })
})
