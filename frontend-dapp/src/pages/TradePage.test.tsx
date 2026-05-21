import '@/test/lightweightChartsJsdomMock'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import TradePage from './TradePage'
import { renderWithProviders } from '@/test-utils'
import * as factory from '@/services/terraclassic/factory'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerPair } from '@/types'

const PAIR = 'terra1pair00000000000000000000000000000001'

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
  getPairPaused: vi.fn().mockResolvedValue(false),
  placeLimitOrder: vi.fn(),
  cancelLimitOrder: vi.fn(),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({ fee_bps: 30, treasury: 'terra1treasury0000000000000000000001' }),
}))

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue(null),
}))

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
  })

  it('sub-desktop workspace uses md two-column grid for tablet portrait (GitLab #146)', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    const workspace = await screen.findByTestId('trade-sub-lg-workspace')
    expect(workspace.className).toContain('md:grid-cols-2')
    expect(workspace.className).toContain('lg:hidden')
  })

  it('order ticket exposes Limit and Market tabs (GitLab #152)', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    const marketTabs = await screen.findAllByTestId('trade-order-tab-market')
    expect(marketTabs.length).toBeGreaterThanOrEqual(1)
    const limitTabs = await screen.findAllByTestId('trade-order-tab-limit')
    expect(limitTabs.length).toBeGreaterThanOrEqual(1)
  })

  it('limit tab shows pre-submit summary before Place limit (GitLab #157)', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })
    const summaries = await screen.findAllByTestId('trade-limit-pre-submit-summary')
    expect(summaries.length).toBeGreaterThanOrEqual(1)
    expect(summaries[0].textContent).toMatch(/no taker slippage/i)
    expect(summaries[0].textContent).toMatch(/Maker placement fee/i)
  })

  it('keeps disconnected ticket wallet CTAs actionable', async () => {
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    const placeButtons = await screen.findAllByTestId('trade-limit-submit')
    expect(placeButtons.some((button) => !button.hasAttribute('disabled'))).toBe(true)

    const cancelButtons = await screen.findAllByTestId('trade-cancel-submit')
    expect(cancelButtons.some((button) => !button.hasAttribute('disabled'))).toBe(true)
  })

  it('shows accurate indexer outage banner when pair fetch fails (GitLab #164)', async () => {
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<TradePage />, { route: `/trade/${PAIR}` })

    const banner = await screen.findByTestId('trade-indexer-outage-banner')
    expect(banner.textContent).toMatch(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/still use chain|VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(banner.textContent).toMatch(/order book|chart|tape/i)
  })
})
