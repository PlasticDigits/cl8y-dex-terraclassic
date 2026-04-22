import { describe, it, expect, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import ChartsPage from './ChartsPage'
import { renderWithProviders } from '@/test-utils'
import { CHARTS_INTEGRATION_PAIR_ADDRESS } from '@/test/chartsIntegrationConstants'
import { getCandles, getOverview, INDEXER_URL } from '@/services/indexer/client'

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

describe('charts stack integration (indexer HTTP)', () => {
  it('uses configured INDEXER_URL', () => {
    expect(INDEXER_URL).toMatch(/^https?:\/\//)
  })

  it('GET /api/v1/overview succeeds', async () => {
    const overview = await getOverview()
    expect(overview.pair_count).toBeGreaterThanOrEqual(1)
    expect(overview.token_count).toBeGreaterThanOrEqual(1)
  })

  it('GET /api/v1/pairs/{addr}/candles returns candles for seeded pair', async () => {
    const rows = await getCandles(CHARTS_INTEGRATION_PAIR_ADDRESS, '1h')
    expect(rows.length).toBeGreaterThan(0)
    expect(rows[0]).toMatchObject({
      open_time: expect.any(String),
      open: expect.any(String),
      high: expect.any(String),
      low: expect.any(String),
      close: expect.any(String),
    })
  })

  it('GET candles for unknown pair surfaces 404 via client error', async () => {
    await expect(getCandles('terra1nonexistent', '1h')).rejects.toThrow(/404/)
  })
})

describe('ChartsPage integration (live indexer)', () => {
  it('renders charts shell without indexer-down banner', async () => {
    renderWithProviders(<ChartsPage />, { route: '/charts' })
    await waitFor(() => expect(screen.getByText(/charts & analytics/i)).toBeInTheDocument(), {
      timeout: 60_000,
    })
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    await waitFor(() => expect(screen.getByRole('heading', { name: /price chart/i })).toBeInTheDocument(), {
      timeout: 60_000,
    })
  })
})
