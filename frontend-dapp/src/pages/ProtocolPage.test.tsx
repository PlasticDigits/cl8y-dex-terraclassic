import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, within, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import ProtocolPage from './ProtocolPage'
import * as indexerClient from '@/services/indexer/client'
import { copyToClipboard } from '@/utils/copyToClipboard'
import {
  PROTOCOL_TRADES_24H_LABEL,
  PROTOCOL_VOLUME_24H_LABEL,
  PROTOCOL_VOLUME_7D_LABEL,
  PROTOCOL_VOLUME_30D_LABEL,
  TRAILING_24H_VOLUME_TITLE,
  TRAILING_7D_VOLUME_TITLE,
  TRAILING_30D_VOLUME_TITLE,
} from '@/utils/trailingWindowCopy'

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
    getOracleVenusVfdusd: vi.fn(),
    getHookEvents: vi.fn(),
    getOverview: vi.fn(),
    getHubPrices: vi.fn(),
    getProtocolFees: vi.fn(),
    getProtocolVolumeDaily: vi.fn(),
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
  total_liquidity_usd: '8900.25',
  liquidity_change_24h_pct: '-3.5',
  liquidity_change_30d_pct: '12.5',
  total_fees_24h_usd: '12.5',
  total_fees_7d_usd: '40',
  total_fees_30d_usd: '100',
  fees_change_24h_pct: '50',
  fees_change_7d_pct: '0',
  fees_change_30d_pct: null,
  volume_change_24h_pct: '25',
  volume_change_7d_pct: '-10',
  volume_change_30d_pct: null,
}

const dailyOk = {
  days: 7,
  timezone: 'UTC',
  methodology: 'protocol_catalog',
  series: [
    { utc_day: '2026-08-20', volume_usd: '10', trade_count: 1 },
    { utc_day: '2026-08-21', volume_usd: '20', trade_count: 2 },
    { utc_day: '2026-08-22', volume_usd: '0', trade_count: 0 },
    { utc_day: '2026-08-23', volume_usd: null, trade_count: 3 },
    { utc_day: '2026-08-24', volume_usd: '15', trade_count: 1 },
    { utc_day: '2026-08-25', volume_usd: '18', trade_count: 1 },
    { utc_day: '2026-08-26', volume_usd: '22', trade_count: 2 },
  ],
}

const feesOk = {
  window: '24h' as const,
  wrap_mapper_configured: true,
  ust1_window_configured: true,
  by_source: [
    { source: 'swap_amm', amount_usd: '8', share_pct: '64', event_count: 2 },
    { source: 'book_take', amount_usd: '2', share_pct: '16', event_count: 1 },
    { source: 'limit_place', amount_usd: '1.5', share_pct: '12', event_count: 1 },
    { source: 'wrap', amount_usd: '1', share_pct: '8', event_count: 1 },
    { source: 'unwrap', amount_usd: '0', share_pct: null, event_count: 0 },
    { source: 'ust1_mint', amount_usd: '0.4', share_pct: '3', event_count: 1 },
    { source: 'ust1_redeem', amount_usd: '0', share_pct: null, event_count: 0 },
  ],
  by_token: [
    { asset_id: 1, symbol: 'UST1', amount_human: '10', amount_usd: '9.8', is_other: false },
    { asset_id: 2, symbol: 'cUSTC', amount_human: '400', amount_usd: '2.7', is_other: false },
  ],
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
  vi.mocked(indexerClient.getOracleVenusVfdusd).mockResolvedValue({
    fdusd_per_vfdusd: '0.023',
    source: 'venus_bsc',
    fetched_at: '2026-01-01T00:00:00Z',
    vtoken: '0xC4eF4229FEc74Ccfe17B2bdeF7715fAC740BA0ba',
  })
}

describe('ProtocolPage (GitLab #550 / #378 / #569)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue(overviewOk)
    vi.mocked(indexerClient.getProtocolFees).mockResolvedValue(feesOk)
    vi.mocked(indexerClient.getProtocolVolumeDaily).mockResolvedValue(dailyOk)
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
    const fees = await screen.findByTestId('protocol-fee-stats')
    const hub = await screen.findByTestId('protocol-dex-hub-prices')
    const oracle = await screen.findByTestId('protocol-oracle')
    expect(stats.compareDocumentPosition(fees) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(fees.compareDocumentPosition(hub) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(hub.compareDocumentPosition(oracle) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    const liq = within(stats).getByTestId('protocol-stat-liquidity')
    expect(liq).toHaveTextContent('$')
    expect(within(liq).getByTestId('protocol-stat-liquidity-24h')).toHaveTextContent('%')
    expect(within(liq).getByTestId('protocol-stat-liquidity-24h')).toHaveTextContent('-')
    expect(within(liq).getByTestId('protocol-stat-liquidity-30d')).toHaveTextContent('+')
    expect(stats.querySelector('.card-glass')).toBeNull()
    expect(within(stats).getByTestId('protocol-stat-volume-24h')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-volume-7d')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-volume-30d')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-tokens')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-tokens')).toHaveTextContent('15')
    expect(within(stats).getByTestId('protocol-stat-tokens').textContent).not.toMatch(/15\.00/)
    expect(within(stats).getByTestId('protocol-stat-tokens-added')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-volume-24h').querySelector('.stat-value-row')).toBeTruthy()
    expect(
      within(stats).getByTestId('protocol-stat-volume-24h').querySelector('.stat-value-row')?.className
    ).not.toMatch(/justify-between/)
    expect(within(stats).getByTestId('protocol-stat-pairs-added')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-active-pairs')).toBeInTheDocument()
    expect(within(stats).getByTestId('protocol-stat-trades-24h')).toBeInTheDocument()
    expect(screen.queryByText('999999')).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /Recent USTC\/USD history/i })).not.toBeInTheDocument()
    expect(screen.getAllByTestId('protocol-oracle')).toHaveLength(1)
  })

  it('renders census integers without trailing decimals (GitLab #667)', async () => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue({
      ...overviewOk,
      token_count: 14,
      tokens_added_30d: 8,
      pairs_added_30d: 7,
      active_pairs_24h: 5,
      total_trades_24h: 151,
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const stats = await screen.findByTestId('protocol-global-stats')
    await waitFor(() => {
      expect(within(stats).getByTestId('protocol-stat-tokens')).toHaveTextContent('14')
    })
    expect(within(stats).getByTestId('protocol-stat-tokens').textContent).not.toMatch(/14\.00/)
    expect(within(stats).getByTestId('protocol-stat-tokens-added')).toHaveTextContent('8')
    expect(within(stats).getByTestId('protocol-stat-tokens-added').textContent).not.toMatch(/8\.000/)
    expect(within(stats).getByTestId('protocol-stat-pairs-added')).toHaveTextContent('7')
    expect(within(stats).getByTestId('protocol-stat-active-pairs')).toHaveTextContent('5')
    expect(within(stats).getByTestId('protocol-stat-trades-24h')).toHaveTextContent('151')
    expect(within(stats).getByTestId('protocol-stat-tokens').querySelector('.stat-delta-cluster')).toBeNull()
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
    expect(vol24.textContent).not.toMatch(/resets at 00:00|always-on/i)
    expect(stats.textContent).not.toMatch(/VITE_INDEXER_URL|https?:\/\//i)
    expect(within(vol24).getByTestId('protocol-stat-volume-24h-chg')).toHaveTextContent('%')
    expect(within(vol7d).getByTestId('protocol-stat-volume-7d-chg')).toHaveTextContent('%')
    expect(within(vol30d).getByTestId('protocol-stat-volume-30d-chg')).toHaveTextContent(/—/)
  })

  it('defaults to USTC and loads price plus history for that ticker', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    expect(await screen.findByRole('heading', { name: /USTC \/ USD/i })).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('ustc')
    expect(indexerClient.getOracleHistory).toHaveBeenCalledWith({ ticker: 'ustc', limit: 48 })
    expect(screen.getByTestId('protocol-oracle-tab-ustc')).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByText('Reference price')).toBeInTheDocument()
    expect(screen.queryByTestId('protocol-oracle-vfdusd-venus')).not.toBeInTheDocument()
    expect(indexerClient.getOracleVenusVfdusd).not.toHaveBeenCalled()
  })

  it('clicking LUNC refetches that ticker and updates the heading', async () => {
    const user = userEvent.setup()
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByRole('heading', { name: /USTC \/ USD/i })
    await user.click(screen.getByTestId('protocol-oracle-tab-lunc'))
    expect(await screen.findByRole('heading', { name: /LUNC \/ USD/i })).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('lunc')
    expect(indexerClient.getOracleHistory).toHaveBeenCalledWith({ ticker: 'lunc', limit: 48 })
    expect(screen.getByText('Reference price')).toBeInTheDocument()
    expect(screen.queryByTestId('protocol-oracle-vfdusd-venus')).not.toBeInTheDocument()
  })

  it('clicking vFDUSD uses vfdusd queries and shows FDUSD reference + Venus section', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getOraclePrice).mockImplementation(async (t = 'ustc') => ({
      ticker: t,
      price_usd: t === 'vfdusd' ? '0.87' : '0.00512',
      sources: [{ source: 'test', price_usd: t === 'vfdusd' ? '0.87' : '0.00512', fetched_at: '2026-01-01T00:00:00Z' }],
    }))
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByRole('heading', { name: /USTC \/ USD/i })
    await user.click(screen.getByTestId('protocol-oracle-tab-vfdusd'))
    expect(await screen.findByRole('heading', { name: /^vFDUSD$/i })).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('vfdusd')
    expect(indexerClient.getOracleHistory).toHaveBeenCalledWith({ ticker: 'vfdusd', limit: 48 })
    expect(await screen.findByText('FDUSD reference price')).toBeInTheDocument()
    expect(screen.queryByText('Reference price')).not.toBeInTheDocument()
    const venus = await screen.findByTestId('protocol-oracle-vfdusd-venus')
    expect(within(venus).getByRole('heading', { name: /1 vFDUSD Price/i })).toBeInTheDocument()
    await waitFor(() => {
      expect(screen.getByTestId('protocol-oracle-vfdusd-venus-value')).toHaveTextContent(/0\.023/)
    })
    expect(screen.getByTestId('protocol-oracle-vfdusd-venus-value')).toHaveTextContent(/FDUSD/)
    expect(indexerClient.getOracleVenusVfdusd).toHaveBeenCalled()
  })

  it('deep-links ?ticker=vfdusd without extra click', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=vfdusd' })
    expect(await screen.findByText('FDUSD reference price')).toBeInTheDocument()
    expect(await screen.findByTestId('protocol-oracle-vfdusd-venus')).toBeInTheDocument()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('vfdusd')
  })

  it('rejects fdusd / XSS / path tickers and does not show Venus', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=fdusd' })
    expect(await screen.findByRole('heading', { name: /USTC \/ USD/i })).toBeInTheDocument()
    expect(screen.queryByTestId('protocol-oracle-vfdusd-venus')).not.toBeInTheDocument()

    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=<img src=x onerror=alert(1)>' })
    expect(await screen.findAllByRole('heading', { name: /USTC \/ USD/i })).toBeTruthy()

    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=javascript:alert(1)' })
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('ustc')
    expect(screen.queryByText('<img')).not.toBeInTheDocument()
  })

  it('CEX error keeps Venus row; Venus error keeps CEX row', async () => {
    vi.mocked(indexerClient.getOraclePrice).mockRejectedValue(new Error('Indexer API error: 502'))
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=vfdusd' })
    expect(await screen.findByText(/Failed to load oracle price/i)).toBeInTheDocument()
    expect(await screen.findByTestId('protocol-oracle-vfdusd-venus')).toBeInTheDocument()
    await waitFor(() => {
      expect(screen.getByTestId('protocol-oracle-vfdusd-venus-value')).toHaveTextContent(/0\.023/)
    })

    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({
      ticker: 'vfdusd',
      price_usd: '0.87',
      sources: [{ source: 'test', price_usd: '0.87', fetched_at: '2026-01-01T00:00:00Z' }],
    })
    vi.mocked(indexerClient.getOracleVenusVfdusd).mockRejectedValue(new Error('Indexer API error: 502'))
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=vfdusd' })
    expect(await screen.findByText('FDUSD reference price')).toBeInTheDocument()
    expect(await screen.findByText(/Failed to load Venus rate/i)).toBeInTheDocument()
    expect(screen.getByText('FDUSD reference price').closest('div')?.parentElement).toHaveTextContent(/0\.87|\$/)
  })

  it('Venus zero/NaN/overflow render em-dash, not Infinity or 1.0', async () => {
    vi.mocked(indexerClient.getOracleVenusVfdusd).mockResolvedValue({
      fdusd_per_vfdusd: 'Infinity',
      source: '<script>alert(1)</script>',
      fetched_at: null,
      vtoken: '0xC4eF4229FEc74Ccfe17B2bdeF7715fAC740BA0ba',
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=vfdusd' })
    await waitFor(() => {
      expect(screen.getByTestId('protocol-oracle-vfdusd-venus-value')).toHaveTextContent(/—/)
    })
    expect(screen.queryByText('Infinity')).not.toBeInTheDocument()
    expect(screen.queryByText('1.0 FDUSD')).not.toBeInTheDocument()
    expect(screen.getByText('Venus')).toBeInTheDocument()
    expect(screen.queryByText('<script>')).not.toBeInTheDocument()
  })

  it('oracle tabs are keyboard accessible', async () => {
    const user = userEvent.setup()
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByTestId('protocol-oracle-tabs')
    const ustc = screen.getByTestId('protocol-oracle-tab-ustc')
    ustc.focus()
    await user.keyboard('{ArrowRight}')
    expect(await screen.findByRole('heading', { name: /LUNC \/ USD/i })).toBeInTheDocument()
    await user.keyboard('{ArrowRight}')
    expect(await screen.findByRole('heading', { name: /^vFDUSD$/i })).toBeInTheDocument()
    expect(await screen.findByTestId('protocol-oracle-vfdusd-venus')).toBeInTheDocument()
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
    vi.mocked(indexerClient.getProtocolFees).mockRejectedValue(new Error('Indexer API error: 404'))
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByTestId('protocol-global-stats')
    const vol7d = await screen.findByTestId('protocol-stat-volume-7d')
    await waitFor(() => expect(vol7d).toHaveTextContent(/—/))
    expect(screen.queryByText('NaN')).not.toBeInTheDocument()
    expect(screen.queryByText('undefined')).not.toBeInTheDocument()
    expect(screen.getByTestId('protocol-stat-liquidity')).toHaveTextContent(/—/)
    expect(screen.getByTestId('protocol-stat-liquidity-24h')).toHaveTextContent(/—/)
    expect(screen.getByTestId('protocol-stat-liquidity-30d')).toHaveTextContent(/—/)
    expect(screen.queryByTestId('protocol-fee-stats')).not.toBeInTheDocument()
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

  it('renders fee panel after global stats with source and token tables (GitLab #586)', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const fees = await screen.findByTestId('protocol-fee-stats')
    expect(within(fees).getByTestId('protocol-stat-fees-24h')).toHaveTextContent('$')
    expect(within(fees).getByTestId('protocol-stat-fees-24h')).toHaveTextContent('+')
    expect(
      within(within(fees).getByTestId('protocol-stat-fees-24h')).getByTestId('protocol-stat-fees-24h-chg')
    ).toHaveTextContent('+')
    expect(within(fees).getByTestId('protocol-stat-fees-7d')).toHaveTextContent('$')
    expect(within(fees).getByTestId('protocol-stat-fees-7d-chg')).toHaveTextContent('0%')
    expect(within(fees).getByTestId('protocol-stat-fees-30d')).toHaveTextContent('$')
    expect(within(fees).getByTestId('protocol-stat-fees-30d-chg')).toHaveTextContent(/—/)
    expect(fees.querySelector('.card-glass')).toBeNull()
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('AMM swap')
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('Book take')
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('Limit place')
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('Wrap')
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('UST1 mint')
    expect(within(fees).getByTestId('protocol-fees-by-source')).not.toHaveTextContent('unwrap')
    expect(within(fees).getByTestId('protocol-fees-by-source')).not.toHaveTextContent('UST1 redeem')
    expect(within(fees).getByTestId('protocol-fees-by-source')).not.toHaveTextContent('deposit')
    expect(within(fees).getByTestId('protocol-fees-by-source')).not.toHaveTextContent('withdraw')
    expect(within(fees).getByTestId('protocol-fees-by-token')).toHaveTextContent('UST1')
    expect(within(fees).getByTestId('protocol-fees-by-token')).toHaveTextContent('cUSTC')
    expect(screen.queryByText('Infinity')).not.toBeInTheDocument()
    expect(indexerClient.getProtocolFees).toHaveBeenCalled()
  })

  it('null fee USD and Δ% are em-dash; idle $0 is $0 (GitLab #586)', async () => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue({
      ...overviewOk,
      total_fees_24h_usd: '0',
      total_fees_7d_usd: null,
      total_fees_30d_usd: null,
      fees_change_24h_pct: null,
      fees_change_7d_pct: null,
      fees_change_30d_pct: null,
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const fees = await screen.findByTestId('protocol-fee-stats')
    expect(within(fees).getByTestId('protocol-stat-fees-24h')).toHaveTextContent(/\$0/)
    expect(within(fees).getByTestId('protocol-stat-fees-7d')).toHaveTextContent(/—/)
    expect(within(fees).getByTestId('protocol-stat-fees-24h-chg')).toHaveTextContent(/—/)
    expect(screen.queryByText('Infinity')).not.toBeInTheDocument()
  })

  it('renders XSS symbol as text, not HTML (GitLab #586)', async () => {
    vi.mocked(indexerClient.getProtocolFees).mockResolvedValue({
      ...feesOk,
      by_token: [
        {
          asset_id: 9,
          symbol: '<img onerror>',
          amount_human: '1',
          amount_usd: null,
          is_other: false,
        },
      ],
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const tokens = await screen.findByTestId('protocol-fees-by-token')
    expect(tokens).toHaveTextContent('<img onerror>')
    expect(tokens.querySelector('img')).toBeNull()
  })

  it('shows Unwrap and wrap-fee tokens when event_count > 0 (GitLab #613)', async () => {
    vi.mocked(indexerClient.getProtocolFees).mockResolvedValue({
      ...feesOk,
      by_source: [
        { source: 'wrap', amount_usd: '0.4', share_pct: '40', event_count: 2 },
        { source: 'unwrap', amount_usd: '0.2', share_pct: '20', event_count: 1 },
        { source: 'swap_amm', amount_usd: '0.4', share_pct: '40', event_count: 1 },
      ],
      by_token: [
        { asset_id: 3, symbol: 'uusd', amount_human: '0.02', amount_usd: '0.0001', is_other: false },
        { asset_id: 4, symbol: 'cLUNC', amount_human: '20', amount_usd: '0.001', is_other: false },
      ],
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const fees = await screen.findByTestId('protocol-fee-stats')
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('Wrap')
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('Unwrap')
    expect(within(fees).getByTestId('protocol-fees-by-token')).toHaveTextContent('uusd')
    expect(within(fees).getByTestId('protocol-fees-by-token')).toHaveTextContent('cLUNC')
  })

  it('fee panel ignores ?ticker= (GitLab #586 / P550-2)', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=javascript:alert(1)' })
    await screen.findByTestId('protocol-fee-stats')
    expect(indexerClient.getProtocolFees).toHaveBeenCalled()
    expect(indexerClient.getOraclePrice).toHaveBeenCalledWith('ustc')
  })

  it('omits UST1 mint/redeem when window is unconfigured (GitLab #614)', async () => {
    vi.mocked(indexerClient.getProtocolFees).mockResolvedValue({
      ...feesOk,
      ust1_window_configured: false,
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const fees = await screen.findByTestId('protocol-fee-stats')
    expect(within(fees).getByTestId('protocol-fees-by-source')).toHaveTextContent('AMM swap')
    expect(within(fees).getByTestId('protocol-fees-by-source')).not.toHaveTextContent('UST1 mint')
    expect(within(fees).getByTestId('protocol-fees-by-source')).not.toHaveTextContent('UST1 redeem')
  })

  it('omits UST1 window rows when flag is missing (old indexer, GitLab #614 / PFee-10)', async () => {
    vi.mocked(indexerClient.getProtocolFees).mockResolvedValue({
      window: feesOk.window,
      wrap_mapper_configured: feesOk.wrap_mapper_configured,
      by_source: feesOk.by_source,
      by_token: feesOk.by_token,
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const fees = await screen.findByTestId('protocol-fee-stats')
    expect(within(fees).getByTestId('protocol-fees-by-source')).not.toHaveTextContent('UST1 mint')
  })

  it('inlines volume Δ% and hosts a UTC-day chart (GitLab #652)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const stats = await screen.findByTestId('protocol-global-stats')
    await waitFor(() => {
      expect(within(stats).getByTestId('protocol-stat-volume-24h')).toHaveTextContent('$')
    })
    expect(within(stats).getByTestId('protocol-stat-volume-24h-chg')).toHaveTextContent('+')
    expect(await screen.findByTestId('protocol-volume-daily-chart')).toBeInTheDocument()
    expect(screen.getByTestId('protocol-volume-daily-chart')).toHaveTextContent(/UTC calendar day/)
    expect(screen.queryByTestId('price-chart')).not.toBeInTheDocument()
    expect(indexerClient.getProtocolVolumeDaily).toHaveBeenCalledWith(7)
    await user.click(screen.getByTestId('protocol-volume-daily-30d'))
    await waitFor(() => {
      expect(indexerClient.getProtocolVolumeDaily).toHaveBeenCalledWith(30)
    })
  })

  it('hides the daily chart when the endpoint is missing (GitLab #652)', async () => {
    vi.mocked(indexerClient.getProtocolVolumeDaily).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    await screen.findByTestId('protocol-global-stats')
    await waitFor(() => {
      expect(screen.queryByTestId('protocol-volume-daily-chart')).not.toBeInTheDocument()
    })
    expect(screen.getByTestId('protocol-stat-volume-24h')).toBeInTheDocument()
  })

  it('old indexer without volume_change keys still renders tiles with em-dash Δ% (GitLab #652)', async () => {
    const legacy = { ...overviewOk }
    delete legacy.volume_change_24h_pct
    delete legacy.volume_change_7d_pct
    delete legacy.volume_change_30d_pct
    vi.mocked(indexerClient.getOverview).mockResolvedValue(legacy)
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const vol24 = await screen.findByTestId('protocol-stat-volume-24h')
    await waitFor(() => {
      expect(vol24).toHaveTextContent('$')
    })
    expect(within(vol24).getByTestId('protocol-stat-volume-24h-chg')).toHaveTextContent(/—/)
  })

  it('renders XSS volume and daily labels as text (GitLab #652)', async () => {
    vi.mocked(indexerClient.getOverview).mockResolvedValue({
      ...overviewOk,
      total_volume_24h_usd: '"><script>alert(1)</script>',
      volume_change_24h_pct: 'javascript:alert(1)',
    })
    vi.mocked(indexerClient.getProtocolVolumeDaily).mockResolvedValue({
      ...dailyOk,
      series: [{ utc_day: '"><script>', volume_usd: 'javascript:', trade_count: 1 }],
    })
    renderWithProviders(<ProtocolPage />, { route: '/protocol' })
    const vol24 = await screen.findByTestId('protocol-stat-volume-24h')
    expect(vol24.querySelector('script')).toBeNull()
    expect(document.querySelector('script[src="javascript:"]')).toBeNull()
    expect(screen.queryByText('Infinity')).not.toBeInTheDocument()
  })

  it('CEX oracle card still does not claim to be the UST1 window rate (P550-11)', async () => {
    renderWithProviders(<ProtocolPage />, { route: '/protocol?ticker=vfdusd' })
    const oracle = await screen.findByTestId('protocol-oracle')
    expect(oracle.textContent).not.toMatch(/is the UST1 window|window mint|oracle mint\/redeem/i)
    expect(await screen.findByTestId('protocol-fee-stats')).toBeInTheDocument()
  })
})
