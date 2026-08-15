import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import ChartsPage from './ChartsPage'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import * as oracle from '@/services/terraclassic/oracle'
import type { IndexerPair } from '@/types'

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

vi.mock('@/services/terraclassic/oracle', () => ({
  getTwapPrices: vi.fn().mockResolvedValue([
    { label: '5m', seconds: 300, price: null },
    { label: '1h', seconds: 3600, price: null },
    { label: '24h', seconds: 86400, price: null },
  ]),
  getOracleInfo: vi.fn().mockResolvedValue(null),
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getOverview: vi.fn(),
    getPairs: vi.fn(),
    getPair: vi.fn(),
    getPairStats: vi.fn(),
    getTrades: vi.fn(),
    getLeaderboard: vi.fn(),
    getCandles: vi.fn(),
    getOraclePrice: vi.fn(),
  }
})

const mockPair: IndexerPair = {
  pair_address: 'terra1pair0000000000000000000000000000ab',
  asset_0: { symbol: 'AAA', contract_addr: 'terra1aaa', denom: null, decimals: 6 },
  asset_1: { symbol: 'BBB', contract_addr: 'terra1bbb', denom: null, decimals: 6 },
  lp_token: 'terra1lp0000000000000000000000000000001',
  fee_bps: 30,
  volume_quote_24h: '1000',
  is_active: true,
}

describe('ChartsPage (component)', () => {
  beforeEach(() => {
    vi.mocked(oracle.getTwapPrices).mockResolvedValue([
      { label: '5m', seconds: 300, price: null },
      { label: '1h', seconds: 3600, price: null },
      { label: '24h', seconds: 86400, price: null },
    ])
    vi.mocked(indexerClient.getOverview).mockResolvedValue({
      total_volume_24h: '0',
      total_volume_24h_usd: '0',
      total_trades_24h: 0,
      pair_count: 1,
      token_count: 2,
      ustc_price_usd: null,
    })
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [mockPair],
      total: 1,
      limit: 50,
      offset: 0,
    })
    vi.mocked(indexerClient.getPairStats).mockResolvedValue({
      volume_base: '1',
      volume_quote: '1',
      trade_count: 1,
      high: '1',
      low: '1',
      open_price: '1',
      close_price: '1',
      price_change_pct: 0,
    })
    vi.mocked(indexerClient.getTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([])
    vi.mocked(indexerClient.getCandles).mockResolvedValue([])
    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({
      ticker: 'ustc',
      price_usd: '0.004928',
      sources: [],
    })
  })

  it('shows retail market-data banner when overview and pairs fail with transport errors (GitLab #215)', async () => {
    vi.mocked(indexerClient.getOverview).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getPairs).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<ChartsPage />)
    const banner = await screen.findByTestId('charts-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })

  it('passes active pair into getCandles via PriceChart and wraps it in a bounded-height container (GitLab #151 /charts follow-up)', async () => {
    const { container } = renderWithProviders(<ChartsPage />)
    await waitFor(() =>
      expect(indexerClient.getCandles).toHaveBeenCalledWith(mockPair.pair_address, expect.any(String))
    )
    expect(container.innerHTML).toContain('h-[min(70vh,720px)]')
  })

  it('shows empty pairs copy when indexer returns no pairs', async () => {
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [],
      total: 0,
      limit: 50,
      offset: 0,
    })
    renderWithProviders(<ChartsPage />)
    await waitFor(() => expect(screen.getByText(/no pairs yet/i)).toBeInTheDocument())
  })
})
