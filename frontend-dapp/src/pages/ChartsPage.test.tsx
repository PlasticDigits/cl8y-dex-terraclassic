import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import { Route, Routes } from 'react-router-dom'
import ChartsPage from './ChartsPage'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import * as oracle from '@/services/terraclassic/oracle'
import type { IndexerPair } from '@/types'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))

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
    expect(screen.queryByTestId('pair-token-links')).not.toBeInTheDocument()
  })

  describe('overview strip (GitLab #548)', () => {
    it('F1/F10: one USD volume box, no raw 10,000,000T', async () => {
      vi.mocked(indexerClient.getOverview).mockResolvedValue({
        total_volume_24h: '10000000000000000000',
        total_volume_24h_usd: '1234.56',
        total_trades_24h: 4,
        pair_count: 13,
        token_count: 12,
        ustc_price_usd: '0.004878',
      })
      renderWithProviders(<ChartsPage />)
      const vol = await screen.findByTestId('charts-overview-volume-usd')
      await waitFor(() => expect(vol).toHaveTextContent('$1.235K'))
      expect(vol).toHaveTextContent(/24h Volume \(USD\)/i)
      expect(screen.queryByTestId('charts-overview-volume-raw')).not.toBeInTheDocument()
      expect(document.body.textContent).not.toMatch(/10,000,000T/)
      expect(screen.queryByText('24h Volume')).not.toBeInTheDocument()
      expect(screen.getByTestId('charts-overview-ustc-usd')).toHaveTextContent('$')
      expect(screen.getByTestId('charts-overview-ustc-usd').textContent).not.toMatch(/\dT\b/)
      expect(screen.getByTestId('charts-overview-trades')).toHaveTextContent('4')
      expect(screen.getByTestId('charts-overview-pairs')).toHaveTextContent('13')
      expect(screen.getByTestId('charts-overview-tokens')).toHaveTextContent('12')
    })

    it('F2: unpriced USD with trades shows em dash not 0', async () => {
      vi.mocked(indexerClient.getOverview).mockResolvedValue({
        total_volume_24h: '1000',
        total_volume_24h_usd: '0',
        total_trades_24h: 4,
        pair_count: 1,
        token_count: 2,
        ustc_price_usd: null,
      })
      renderWithProviders(<ChartsPage />)
      const vol = await screen.findByTestId('charts-overview-volume-usd')
      await waitFor(() => expect(vol).toHaveTextContent('—'))
      expect(vol.textContent).not.toMatch(/\$0/)
    })

    it('F3: idle DEX volume is $0', async () => {
      vi.mocked(indexerClient.getOverview).mockResolvedValue({
        total_volume_24h: '0',
        total_volume_24h_usd: '0',
        total_trades_24h: 0,
        pair_count: 1,
        token_count: 2,
        ustc_price_usd: null,
      })
      renderWithProviders(<ChartsPage />)
      const vol = await screen.findByTestId('charts-overview-volume-usd')
      await waitFor(() => expect(vol).toHaveTextContent('$0'))
    })

    it('F5: missing USTC spot is em dash', async () => {
      vi.mocked(indexerClient.getOverview).mockResolvedValue({
        total_volume_24h: '0',
        total_volume_24h_usd: '0',
        total_trades_24h: 0,
        pair_count: 1,
        token_count: 2,
        ustc_price_usd: '',
      })
      renderWithProviders(<ChartsPage />)
      const box = await screen.findByTestId('charts-overview-ustc-usd')
      await waitFor(() => expect(box).toHaveTextContent('—'))
    })

    it('F9: adversarial USD field does not inject HTML', async () => {
      vi.mocked(indexerClient.getOverview).mockResolvedValue({
        total_volume_24h: '1',
        total_volume_24h_usd: '"><script>alert(1)</script>',
        total_trades_24h: 1,
        pair_count: 1,
        token_count: 2,
        ustc_price_usd: null,
      })
      renderWithProviders(<ChartsPage />)
      const vol = await screen.findByTestId('charts-overview-volume-usd')
      await waitFor(() => expect(vol).toHaveTextContent('—'))
      expect(vol.querySelector('script')).toBeNull()
    })
  })

  describe('token identity (GitLab #541)', () => {
    const PAIR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'
    const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
    const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'

    it('U5: shows identity row for a selected checksummed pair', async () => {
      const identityPair: IndexerPair = {
        ...mockPair,
        pair_address: PAIR,
        asset_0: { symbol: 'UST1', contract_addr: UST1, denom: null, decimals: 6 },
        asset_1: { symbol: 'cUSTC', contract_addr: CUSTC, denom: null, decimals: 6 },
      }
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [identityPair],
        total: 1,
        limit: 50,
        offset: 0,
      })
      renderWithProviders(<ChartsPage />)
      expect(await screen.findByTestId('pair-token-links')).toBeInTheDocument()
      expect(screen.getByTestId('token-identity-base')).toHaveAttribute('data-identity-payload', UST1)
      expect(screen.getByTestId('token-identity-quote')).toHaveAttribute('data-identity-payload', CUSTC)
      expect(screen.getByTestId('token-identity-pair')).toBeInTheDocument()
    })
  })

  describe('pair deep link (GitLab #547)', () => {
    const DEEP = 'terra1pair0000000000000000000000000000000001'

    function renderCharts(route: string) {
      return renderWithProviders(
        <Routes>
          <Route path="/charts" element={<ChartsPage />} />
          <Route path="/charts/:pairAddr" element={<ChartsPage />} />
        </Routes>,
        { route }
      )
    }

    it('H2: valid :pairAddr selects that pair', async () => {
      const deepPair: IndexerPair = { ...mockPair, pair_address: DEEP }
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [mockPair, deepPair],
        total: 2,
        limit: 50,
        offset: 0,
      })
      renderCharts(`/charts/${DEEP}`)
      await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(DEEP, expect.any(String)))
    })

    it('H3: invalid param shows a notice and does not fetch the junk segment', async () => {
      vi.mocked(indexerClient.getPair).mockClear()
      renderCharts('/charts/not-a-terra')
      expect(await screen.findByTestId('charts-invalid-pair-notice')).toBeInTheDocument()
      expect(indexerClient.getPair).not.toHaveBeenCalledWith('not-a-terra')
      expect(document.body.innerHTML).not.toMatch(/javascript:/)
    })

    it('H4: unknown but valid terra1 does not crash', async () => {
      vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('not found'))
      renderCharts(`/charts/${DEEP}`)
      expect(await screen.findByTestId('charts-unknown-pair-notice')).toBeInTheDocument()
    })
  })
})
