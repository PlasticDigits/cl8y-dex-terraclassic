import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Route, Routes } from 'react-router-dom'
import ChartsPage from './ChartsPage'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import * as oracle from '@/services/terraclassic/oracle'
import type { IndexerPair } from '@/types'
import {
  CHARTS_PAIR_SORT_VOLUME_LABEL,
  TRAILING_24H_TRADES_LABEL,
  TRAILING_24H_TRADES_TITLE,
  TRAILING_24H_VOLUME_LABEL,
  TRAILING_24H_VOLUME_TITLE,
} from '@/utils/trailingWindowCopy'

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

vi.mock('@/services/terraclassic/assetCodeIdFreeze', () => ({
  probePairCodeIdFreeze: vi.fn().mockResolvedValue({ frozen: false, verdict: 'tradable' }),
}))

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
      volume_usd: '1',
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
    vi.mocked(indexerClient.getPair).mockImplementation(async (addr: string) => ({
      ...mockPair,
      pair_address: addr,
    }))
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

  it('shows freeze banner when indexer flags code_id_frozen (GitLab #585)', async () => {
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [{ ...mockPair, code_id_frozen: true }],
      total: 1,
      limit: 50,
      offset: 0,
    })
    renderWithProviders(<ChartsPage />)
    expect(await screen.findByTestId('charts-pair-code-id-frozen-banner')).toHaveTextContent(/quotes can still appear/i)
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
      expect(vol).toHaveTextContent(TRAILING_24H_VOLUME_LABEL)
      expect(vol).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
      expect(within(vol).getByText(TRAILING_24H_VOLUME_LABEL)).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
      expect(within(vol).getByLabelText(/last 24 hours, not a midnight reset/i)).toHaveTextContent('$1.235K')
      expect(screen.queryByTestId('charts-overview-volume-raw')).not.toBeInTheDocument()
      expect(document.body.textContent).not.toMatch(/10,000,000T/)
      expect(screen.queryByText('24h Volume')).not.toBeInTheDocument()
      expect(screen.getByTestId('charts-overview-ustc-usd')).toHaveTextContent('$')
      expect(screen.getByTestId('charts-overview-ustc-usd').textContent).not.toMatch(/\dT\b/)
      const trades = screen.getByTestId('charts-overview-trades')
      expect(trades).toHaveTextContent(TRAILING_24H_TRADES_LABEL)
      expect(trades).toHaveAttribute('title', TRAILING_24H_TRADES_TITLE)
      expect(within(trades).getByLabelText(/last 24 hours, not a midnight reset/i)).toHaveTextContent('4')
      expect(screen.getByTestId('charts-overview-pairs')).toHaveTextContent('13')
      expect(screen.getByTestId('charts-overview-tokens')).toHaveTextContent('12')
      expect(vol.className).toMatch(/stat-flat/)
      expect(vol.className).not.toMatch(/card-glass/)
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
      expect(vol).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
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
      expect(vol).toHaveTextContent(TRAILING_24H_VOLUME_LABEL)
      expect(vol).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
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
      expect(vol).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
      expect(vol.getAttribute('title')).not.toMatch(/script|alert/i)
    })

    it('U7: pair SORT default is Last 24h volume (volume_24h)', async () => {
      renderWithProviders(<ChartsPage />)
      await screen.findByTestId('charts-overview-volume-usd')
      expect(screen.getByText(CHARTS_PAIR_SORT_VOLUME_LABEL)).toBeInTheDocument()
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

  describe('v2 LP identity (GitLab #664)', () => {
    const PAIR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'
    const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
    const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'

    it('C1: identity row shows v2 LP when getPair is stamped', async () => {
      const identityPair: IndexerPair = {
        ...mockPair,
        pair_address: PAIR,
        asset_0: { symbol: 'UST1', contract_addr: UST1, denom: null, decimals: 6 },
        asset_1: { symbol: 'cUSTC', contract_addr: CUSTC, denom: null, decimals: 6 },
        liquidity_usd: '1234.5',
      }
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [identityPair],
        total: 1,
        limit: 50,
        offset: 0,
      })
      vi.mocked(indexerClient.getPair).mockResolvedValue(identityPair)
      renderWithProviders(<ChartsPage />)
      const chip = await screen.findByTestId('token-identity-v2-lp-usd')
      expect(chip).toHaveTextContent('v2 LP')
      expect(chip).toHaveTextContent('$')
    })

    it('C2: 24h Stats Vol (USD) is still volume_usd, not TVL', async () => {
      const identityPair: IndexerPair = {
        ...mockPair,
        pair_address: PAIR,
        liquidity_usd: '99999',
      }
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [identityPair],
        total: 1,
        limit: 50,
        offset: 0,
      })
      vi.mocked(indexerClient.getPair).mockResolvedValue(identityPair)
      vi.mocked(indexerClient.getPairStats).mockResolvedValue({
        volume_base: '1',
        volume_quote: '1',
        volume_usd: '42.5',
        trade_count: 2,
        high: '1',
        low: '1',
        open_price: '1',
        close_price: '1',
        price_change_pct: 0,
      })
      renderWithProviders(<ChartsPage />)
      const vol = await screen.findByTestId('charts-pair-volume-usd')
      await waitFor(() => expect(vol).toHaveTextContent('$'))
      expect(vol.textContent).toMatch(/42/)
      expect(vol.textContent).not.toMatch(/99999/)
      expect(screen.getByTestId('charts-pair-24h-stats')).toBeInTheDocument()
    })

    it('C3: empty pairs has no identity LP chip', async () => {
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [],
        total: 0,
        limit: 50,
        offset: 0,
      })
      renderWithProviders(<ChartsPage />)
      await waitFor(() => expect(screen.getByText(/no pairs yet/i)).toBeInTheDocument())
      expect(screen.queryByTestId('token-identity-v2-lp-usd')).not.toBeInTheDocument()
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

  describe('trader leaderboard volume (GitLab #553)', () => {
    const ADDR = 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd'

    it('Volume column is USD compact; raw USTR-scale total_volume is not shown as T', async () => {
      vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([
        {
          address: ADDR,
          total_trades: 4,
          total_volume: '10000000000000000000',
          total_volume_usd: '711.2',
          volume_24h: '0',
          volume_7d: '0',
          volume_30d: '0',
          tier_id: null,
          tier_name: null,
          registered: false,
          first_trade_at: null,
          last_trade_at: null,
          total_realized_pnl: '0',
          best_trade_pnl: null,
          worst_trade_pnl: null,
          total_fees_paid: '0',
        },
      ])
      renderWithProviders(<ChartsPage />)
      await waitFor(() => expect(indexerClient.getLeaderboard).toHaveBeenCalledWith('total_volume_usd', 20))
      const cell = await screen.findByTestId('charts-leaderboard-volume')
      expect(cell.textContent).toMatch(/\$/)
      expect(cell.textContent).not.toMatch(/10,000,000T/)
      expect(cell.textContent).not.toMatch(/\dT\b/)
      expect(screen.getByRole('tab', { name: /volume \(usd\)/i })).toHaveAttribute('aria-selected', 'true')
    })

    it('unpriced leaderboard volume is an em dash, not $0', async () => {
      vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([
        {
          address: ADDR,
          total_trades: 2,
          total_volume: '10000000000000000000',
          total_volume_usd: null,
          volume_24h: '0',
          volume_7d: '0',
          volume_30d: '0',
          tier_id: null,
          tier_name: null,
          registered: false,
          first_trade_at: null,
          last_trade_at: null,
          total_realized_pnl: '0',
          best_trade_pnl: null,
          worst_trade_pnl: null,
          total_fees_paid: '0',
        },
      ])
      renderWithProviders(<ChartsPage />)
      const cell = await screen.findByTestId('charts-leaderboard-volume')
      expect(cell).toHaveTextContent('—')
      expect(cell.textContent).not.toMatch(/\$0/)
    })
  })

  describe('pair 24h stats volume (GitLab #565)', () => {
    const ust1Custc: IndexerPair = {
      ...mockPair,
      asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
      asset_1: { symbol: 'cUSTC', contract_addr: 'terra1custc', denom: null, decimals: 6 },
    }

    async function renderPairStats(
      pair: IndexerPair,
      stats: {
        volume_base: string
        volume_quote: string
        volume_usd?: string | null
        trade_count: number
      }
    ) {
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [pair],
        total: 1,
        limit: 50,
        offset: 0,
      })
      vi.mocked(indexerClient.getPairStats).mockResolvedValue({
        volume_base: stats.volume_base,
        volume_quote: stats.volume_quote,
        volume_usd: stats.volume_usd,
        trade_count: stats.trade_count,
        high: '1',
        low: '1',
        open_price: '1',
        close_price: '1',
        price_change_pct: 0,
      })
      renderWithProviders(<ChartsPage />)
      return screen.findByTestId('charts-pair-volume-usd')
    }

    it('V1: UST1/cUSTC primary vol is USD compact, not formatNum(raw)', async () => {
      const usd = await renderPairStats(ust1Custc, {
        volume_base: '847004054',
        volume_quote: '157465643310',
        volume_usd: '763.35',
        trade_count: 41,
      })
      await waitFor(() => expect(usd).toHaveTextContent(/\$/))
      expect(usd).toHaveTextContent(/Vol \(USD\)/i)
      expect(usd).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
      expect(within(usd).getByText(TRAILING_24H_VOLUME_LABEL)).toHaveAttribute('title', TRAILING_24H_VOLUME_TITLE)
      expect(within(usd).getByLabelText(/last 24 hours, not a midnight reset/i)).toBeInTheDocument()
      const strip = screen.getByTestId('charts-pair-24h-stats')
      expect(strip.textContent).not.toMatch(/847\.0M|157\.5B/)
      expect(screen.getByTestId('charts-pair-volume-base')).toHaveTextContent(/Vol \(UST1\)/)
      expect(screen.getByTestId('charts-pair-volume-quote')).toHaveTextContent(/Vol \(cUSTC\)/)
      expect(screen.getByTestId('charts-pair-volume-base').textContent).toMatch(/847/)
      expect(screen.getByTestId('charts-pair-volume-quote').textContent).toMatch(/157/)
    })

    it('V2: UST1/USTR 18-dec quote never compact-formats as T', async () => {
      const ustrPair: IndexerPair = {
        ...mockPair,
        asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
        asset_1: { symbol: 'USTR', contract_addr: 'terra1ustr', denom: null, decimals: 18 },
      }
      const usd = await renderPairStats(ustrPair, {
        volume_base: '1000000',
        volume_quote: '19300000000000000000',
        volume_usd: '19.30',
        trade_count: 2,
      })
      await waitFor(() => expect(usd).toHaveTextContent(/\$/))
      const strip = screen.getByTestId('charts-pair-24h-stats')
      expect(strip.textContent).not.toMatch(/\dT\b/)
      expect(screen.getByTestId('charts-pair-volume-quote').textContent).not.toMatch(/T$/)
    })

    it.each([null, '0', ''] as const)(
      'V4: unpriced USD %j with trades is em dash, not $0 and not raw fallback',
      async (volume_usd) => {
        const usd = await renderPairStats(ust1Custc, {
          volume_base: '847004054',
          volume_quote: '157465643310',
          volume_usd,
          trade_count: 41,
        })
        await waitFor(() => expect(usd).toHaveTextContent('—'))
        expect(usd.textContent).not.toMatch(/\$0/)
        expect(usd.textContent).not.toMatch(/847\.0M/)
      }
    )

    it('idle pair with USD 0 is $0', async () => {
      const usd = await renderPairStats(ust1Custc, {
        volume_base: '0',
        volume_quote: '0',
        volume_usd: '0',
        trade_count: 0,
      })
      await waitFor(() => expect(usd).toHaveTextContent('$0'))
    })

    it.each(['NaN', '"><script>alert(1)</script>', '1e309', `x${'9'.repeat(200)}`, '-12'] as const)(
      'invalid USD %j is em dash without HTML inject',
      async (volume_usd) => {
        const usd = await renderPairStats(ust1Custc, {
          volume_base: '1',
          volume_quote: '1',
          volume_usd,
          trade_count: 3,
        })
        await waitFor(() => expect(usd).toHaveTextContent('—'))
        expect(usd.querySelector('script')).toBeNull()
      }
    )

    it('V3: missing decimals on a token vol box is em dash', async () => {
      const noDec: IndexerPair = {
        ...ust1Custc,
        asset_0: { ...ust1Custc.asset_0, decimals: Number.NaN },
        asset_1: { ...ust1Custc.asset_1, decimals: 999 },
      }
      await renderPairStats(noDec, {
        volume_base: '1000000',
        volume_quote: '1000000',
        volume_usd: '1',
        trade_count: 1,
      })
      const base = await screen.findByTestId('charts-pair-volume-base')
      const quote = screen.getByTestId('charts-pair-volume-quote')
      expect(base).toHaveTextContent('—')
      expect(quote).toHaveTextContent('—')
    })

    it('V5: invert pill does not change Vol (USD) or swap base/quote decimals', async () => {
      const user = userEvent.setup()
      vi.mocked(indexerClient.getCandles).mockResolvedValue([
        {
          open_time: '2024-01-01T12:00:00.000Z',
          open: '1',
          high: '1.1',
          low: '0.9',
          close: '1.05',
          volume_base: '100',
          volume_quote: '105',
          trade_count: 3,
        },
      ])
      const usd = await renderPairStats(ust1Custc, {
        volume_base: '847004054',
        volume_quote: '157465643310',
        volume_usd: '763.35',
        trade_count: 41,
      })
      await waitFor(() => expect(usd).toHaveTextContent(/\$/))
      const beforeUsd = usd.textContent
      const beforeBase = screen.getByTestId('charts-pair-volume-base').textContent
      const beforeQuote = screen.getByTestId('charts-pair-volume-quote').textContent
      await user.click(await screen.findByTestId('trade-pair-invert-pill'))
      expect(screen.getByTestId('charts-pair-volume-usd').textContent).toBe(beforeUsd)
      expect(screen.getByTestId('charts-pair-volume-base').textContent).toBe(beforeBase)
      expect(screen.getByTestId('charts-pair-volume-quote').textContent).toBe(beforeQuote)
      expect(screen.getByTestId('charts-pair-volume-base')).toHaveTextContent(/Vol \(UST1\)/)
      expect(screen.getByTestId('charts-pair-volume-quote')).toHaveTextContent(/Vol \(cUSTC\)/)
    })
  })

  describe('pair 24h stats + TWAP human scale (GitLab #564)', () => {
    const UST1_USTR: IndexerPair = {
      ...mockPair,
      asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
      asset_1: { symbol: 'USTR', contract_addr: 'terra1ustr', denom: null, decimals: 18 },
    }

    const EQUAL_DEC: IndexerPair = {
      ...mockPair,
      pair_address: 'terra1pair0000000000000000000000000000eq',
      asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
      asset_1: { symbol: 'cUSTC', contract_addr: 'terra1custc', denom: null, decimals: 6 },
    }

    it('S1/S2/S4/S5: UST1/USTR vols and TWAP are human, not T/M compact raw', async () => {
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [UST1_USTR],
        total: 1,
        limit: 50,
        offset: 0,
      })
      vi.mocked(indexerClient.getPairStats).mockResolvedValue({
        volume_base: '385800000',
        volume_quote: '36113437940000000000000',
        volume_usd: '264.2',
        trade_count: 15,
        high: '111',
        low: '90',
        open_price: '95',
        close_price: '111',
        price_change_pct: 16.8,
        high_usd: '0.97',
        low_usd: '0.682427',
        open_price_usd: '0.70',
        close_price_usd: '0.90',
      })
      vi.mocked(oracle.getTwapPrices).mockResolvedValue([
        { label: '5m', seconds: 300, price: '111009000000000' },
        { label: '1h', seconds: 3600, price: '111009000000000' },
        { label: '24h', seconds: 86400, price: '94998200000000' },
      ])
      renderWithProviders(<ChartsPage />)
      const volBase = await screen.findByTestId('charts-pair-volume-base')
      await waitFor(() => expect(volBase).toHaveTextContent(/385\.8/))
      expect(volBase.textContent).not.toMatch(/M$/)
      expect(volBase).toHaveTextContent(/Vol \(UST1\)/)
      const volQuote = screen.getByTestId('charts-pair-volume-quote')
      expect(volQuote.textContent).toMatch(/K$/)
      expect(volQuote.textContent).not.toMatch(/T$/)
      expect(volQuote).toHaveTextContent(/Vol \(USTR\)/)
      expect(screen.getByTestId('charts-pair-volume-usd')).toHaveTextContent('$')
      const twap5m = screen.getByTestId('charts-twap-5m')
      expect(twap5m).toHaveTextContent(/111/)
      expect(twap5m.textContent).not.toMatch(/T$/)
      expect(twap5m).toHaveTextContent(/TWAP 5m/)
      expect(twap5m.textContent).not.toMatch(/\dT\b/)
      expect(document.body.textContent).not.toMatch(/36,113,437,940T/)
      expect(volBase.textContent).not.toMatch(/385\.8M/)
      expect(screen.getByTestId('charts-pair-low-usd')).toHaveTextContent('$0.682427')
      expect(screen.getByTestId('charts-pair-low-usd').textContent).not.toMatch(/[TMBK]/)
    })

    it('S3: 6/6 pair vols stay readable without extra 1e6 scale', async () => {
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: [EQUAL_DEC],
        total: 1,
        limit: 50,
        offset: 0,
      })
      vi.mocked(indexerClient.getPairStats).mockResolvedValue({
        volume_base: '1000000',
        volume_quote: '206000000',
        volume_usd: '206',
        trade_count: 2,
        high: '1',
        low: '1',
        open_price: '1',
        close_price: '1',
        price_change_pct: 0,
        high_usd: '1',
        low_usd: '1',
        open_price_usd: '1',
        close_price_usd: '1',
      })
      vi.mocked(oracle.getTwapPrices).mockResolvedValue([
        { label: '5m', seconds: 300, price: '1.05' },
        { label: '1h', seconds: 3600, price: '1.05' },
        { label: '24h', seconds: 86400, price: '1.05' },
      ])
      renderWithProviders(<ChartsPage />)
      const volBase = await screen.findByTestId('charts-pair-volume-base')
      await waitFor(() => expect(volBase.textContent).toMatch(/1(\.0+)?/))
      expect(screen.getByTestId('charts-pair-volume-quote').textContent).toMatch(/^[\s\S]*206/)
      expect(screen.getByTestId('charts-twap-5m')).toHaveTextContent('1.05')
      expect(screen.getByTestId('charts-twap-5m').textContent).not.toMatch(/T$/)
    })

    it('S4: unpriced volume_usd with trades is em dash', async () => {
      vi.mocked(indexerClient.getPairStats).mockResolvedValue({
        volume_base: '1000000',
        volume_quote: '1000000',
        volume_usd: null,
        trade_count: 15,
        high: '1',
        low: '1',
        open_price: '1',
        close_price: '1',
        price_change_pct: 0,
      })
      renderWithProviders(<ChartsPage />)
      const usd = await screen.findByTestId('charts-pair-volume-usd')
      await waitFor(() => expect(usd).toHaveTextContent('—'))
      expect(usd.textContent).not.toMatch(/\$0/)
    })

    it('S4: idle pair 24h USD is $0', async () => {
      vi.mocked(indexerClient.getPairStats).mockResolvedValue({
        volume_base: '0',
        volume_quote: '0',
        volume_usd: '0',
        trade_count: 0,
        high: null,
        low: null,
        open_price: null,
        close_price: null,
        price_change_pct: null,
      })
      renderWithProviders(<ChartsPage />)
      const usd = await screen.findByTestId('charts-pair-volume-usd')
      await waitFor(() => expect(usd).toHaveTextContent('$0'))
    })

    it('S11: oracle observe failure shows TWAP em dash and unavailable copy', async () => {
      vi.mocked(oracle.getTwapPrices).mockRejectedValue(new Error('lcd down'))
      renderWithProviders(<ChartsPage />)
      const twap = await screen.findByTestId('charts-twap-5m')
      await waitFor(() => expect(twap).toHaveTextContent('—'))
      expect(await screen.findByText(/oracle data unavailable for this pair/i)).toBeInTheDocument()
      expect(screen.getByTestId('charts-pair-volume-base')).toBeInTheDocument()
    })
  })
})
