import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, within, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import ProtocolPage from './ProtocolPage'
import * as indexerClient from '@/services/indexer/client'
import { copyToClipboard } from '@/utils/copyToClipboard'

const CUSTC_WRAP = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
const CLUNC_WRAP = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FACTORY_CONTRACT_ADDRESS: 'terra1factory000000000000000000000000001',
    ROUTER_CONTRACT_ADDRESS: 'terra1router00000000000000000000000000001',
    TERRA_LCD_URL: 'http://localhost:1317',
    TERRA_RPC_URL: 'http://localhost:26657',
    USTC_C_TOKEN_ADDRESS: 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch',
    LUNC_C_TOKEN_ADDRESS: 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg',
  }
})

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue({ ok: true }),
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
  tickers: ['custc', 'lunc', 'ust1', 'ustr'],
  prices: [
    {
      ticker: 'custc',
      price_usd: '0.005',
      source_pair: null,
      asset_address: CUSTC_WRAP,
      tvl_usd: null,
      updated_at: '2026-01-01T00:00:00Z',
    },
    {
      ticker: 'lunc',
      price_usd: '0.00008',
      source_pair: null,
      asset_address: CLUNC_WRAP,
      tvl_usd: null,
      updated_at: '2026-01-01T00:00:00Z',
    },
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

  it('renders DEX hub card for cUSTC / LUNC / UST1 / USTR and never queries CEX ustr', async () => {
    const user = userEvent.setup()
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const hub = await screen.findByTestId('protocol-dex-hub-prices')
    await waitFor(() => expect(within(hub).getByTestId('protocol-dex-hub-custc-usd')).toHaveTextContent('$'))
    expect(within(hub).getByTestId('protocol-dex-hub-lunc-usd')).toHaveTextContent('$')
    expect(within(hub).getByTestId('protocol-dex-hub-lunc-usd').textContent).not.toMatch(/T$/)
    expect(within(hub).getByTestId('protocol-dex-hub-ust1-usd')).toHaveTextContent('$')
    expect(within(hub).getByTestId('protocol-dex-hub-ustr-usd')).toHaveTextContent('$')
    expect(hub).toHaveTextContent(/DEX reference — not CEX, not settlement/i)
    expect(within(hub).getByTestId('protocol-dex-hub-custc-token')).toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-lunc-token')).toBeInTheDocument()
    expect(within(hub).queryByTestId('protocol-dex-hub-custc-source')).not.toBeInTheDocument()
    expect(within(hub).queryByTestId('protocol-dex-hub-lunc-source')).not.toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-ust1-source')).toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-ustr-source')).toBeInTheDocument()
    expect(within(hub).getByLabelText('Copy UST1 source pair')).toBeInTheDocument()
    expect(within(hub).getByLabelText('Copy cUSTC token contract')).toBeInTheDocument()
    expect(within(hub).getByLabelText('Copy cLUNC wrap contract')).toBeInTheDocument()
    await user.click(within(hub).getByLabelText('Copy cUSTC token contract'))
    expect(copyToClipboard).toHaveBeenCalledWith(CUSTC_WRAP)
    await user.click(within(hub).getByLabelText('Copy cLUNC wrap contract'))
    expect(copyToClipboard).toHaveBeenCalledWith(CLUNC_WRAP)
    const custcExplorer = within(hub).getByTestId('protocol-dex-hub-custc-token-explorer')
    expect(custcExplorer).toHaveAttribute('rel', expect.stringContaining('noopener'))
    expect(custcExplorer.getAttribute('href')).not.toMatch(/javascript:|data:/)
    expect(indexerClient.getHubPrices).toHaveBeenCalled()
    expect(indexerClient.getOraclePrice).not.toHaveBeenCalledWith('ustr')
    expect(indexerClient.getOraclePrice).not.toHaveBeenCalledWith('ust1')
    expect(indexerClient.getOraclePrice).not.toHaveBeenCalledWith('custc')
    expect(screen.getByTestId('protocol-oracle-tabs').querySelectorAll('[role="tab"]')).toHaveLength(3)
  })

  it('null hub prices render em-dash, not $0 or $1', async () => {
    vi.mocked(indexerClient.getHubPrices).mockResolvedValue({
      metadata: 'DEX hub USD marks. Not CEX.',
      tickers: ['custc', 'lunc', 'ust1', 'ustr'],
      prices: [
        { ticker: 'custc', price_usd: null },
        { ticker: 'lunc', price_usd: null },
        { ticker: 'ust1', price_usd: null },
        { ticker: 'ustr', price_usd: null },
      ],
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await waitFor(() => expect(screen.getByTestId('protocol-dex-hub-ustr-usd')).toHaveTextContent('—'))
    expect(screen.getByTestId('protocol-dex-hub-ust1-usd')).toHaveTextContent('—')
    expect(screen.getByTestId('protocol-dex-hub-lunc-usd')).toHaveTextContent('—')
    expect(screen.getByTestId('protocol-dex-hub-custc-usd')).toHaveTextContent('—')
    expect(screen.queryByTestId('protocol-dex-hub-ustr-usd')?.textContent).not.toMatch(/\$0|\$1|2\.5/)
    expect(screen.getByTestId('protocol-dex-hub-lunc-usd').textContent).not.toMatch(/\$0|\$1/)
  })

  it('hub 502 shows outage banner without host:port leak', async () => {
    vi.mocked(indexerClient.getHubPrices).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const banner = await screen.findByTestId('protocol-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1|:3001/i)
    const hub = await screen.findByTestId('protocol-dex-hub-prices')
    expect(hub).toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-custc')).toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-lunc')).toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-ust1')).toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-ustr')).toBeInTheDocument()
    await waitFor(() => expect(within(hub).getByTestId('protocol-dex-hub-lunc-usd')).toHaveTextContent('—'))
    expect(within(hub).getByTestId('protocol-dex-hub-custc-usd')).toHaveTextContent('—')
    expect(within(hub).getByTestId('protocol-dex-hub-custc-token')).toBeInTheDocument()
    expect(within(hub).getByTestId('protocol-dex-hub-lunc-token')).toBeInTheDocument()
    expect(within(hub).queryByTestId('protocol-dex-hub-ust1-source')).not.toBeInTheDocument()
  })
})
