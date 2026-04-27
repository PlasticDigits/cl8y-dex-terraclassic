import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import PriceChart from '../PriceChart'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerCandle } from '@/types'
import { lwChartTestDouble } from '@/test/lightweightChartsJsdomMock'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getCandles: vi.fn(),
    getPairStats: vi.fn(),
  }
})

function candle(overrides: Partial<IndexerCandle> = {}): IndexerCandle {
  return {
    open_time: '2024-01-01T12:00:00.000Z',
    open: '1',
    high: '1.1',
    low: '0.9',
    close: '1.05',
    volume_base: '100',
    volume_quote: '105',
    trade_count: 3,
    ...overrides,
  }
}

const pairA = 'terra1pair00000000000000000000000000000aa'
const pairB = 'terra1pair00000000000000000000000000000bb'

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

describe('PriceChart', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getCandles).mockReset()
    vi.mocked(indexerClient.getCandles).mockResolvedValue([candle()])
    vi.mocked(indexerClient.getPairStats).mockReset()
    vi.mocked(indexerClient.getPairStats).mockResolvedValue({ ...emptyStats })
  })

  it('shows loading then renders chart chrome when data resolves', async () => {
    vi.mocked(indexerClient.getCandles).mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve([candle()]), 40))
    )
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    expect(screen.getByText(/loading chart/i)).toBeInTheDocument()
    await waitFor(() => expect(screen.queryByText(/loading chart/i)).not.toBeInTheDocument())
    expect(screen.getByRole('heading', { name: /price \(usd\)/i })).toBeInTheDocument()
  })

  it('shows error banner when getCandles rejects', async () => {
    vi.mocked(indexerClient.getCandles).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(screen.getByText(/failed to load chart data/i)).toBeInTheDocument())
  })

  it('shows accessible empty state when getCandles returns an empty list', async () => {
    vi.mocked(indexerClient.getCandles).mockResolvedValue([])
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(screen.queryByText(/loading chart/i)).not.toBeInTheDocument())
    expect(screen.queryByText(/failed to load chart data/i)).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { name: /price \(usd\)/i })).toBeInTheDocument()
    expect(screen.getByText(/no chart data for this interval yet/i)).toBeInTheDocument()
    expect(
      screen.getByRole('img', {
        name: /no price chart data for this interval/i,
      })
    ).toBeInTheDocument()
    expect(screen.queryByTestId('price-chart-lightweight-canvas')).not.toBeInTheDocument()
    await waitFor(() => expect(indexerClient.getPairStats).toHaveBeenCalledWith(pairA))
  })

  it('shows optional 24h close from getPairStats when candles are empty', async () => {
    vi.mocked(indexerClient.getCandles).mockResolvedValue([])
    vi.mocked(indexerClient.getPairStats).mockResolvedValue({
      ...emptyStats,
      close_price: '1.234567',
    })
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(screen.getByText(/1\.234567/)).toBeInTheDocument())
  })

  it('shows empty state when all candles lack open/close', async () => {
    vi.mocked(indexerClient.getCandles).mockResolvedValue([
      {
        open_time: '2024-01-01T12:00:00.000Z',
        open: '',
        high: '1',
        low: '1',
        close: '',
        volume_base: '0',
        volume_quote: '0',
        trade_count: 0,
      },
    ])
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(screen.getByText(/no chart data for this interval yet/i)).toBeInTheDocument())
  })

  it('requests candles with default interval 1h', async () => {
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(pairA, '1h'))
  })

  it('requests new candles when interval button is pressed', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getCandles).mockResolvedValue([candle()])
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(pairA, '1h'))
    await user.click(screen.getByRole('button', { name: '1d' }))
    await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(pairA, '1d'))
  })

  it('refetches when pairAddress prop changes', async () => {
    const { rerender } = renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(pairA, '1h'))
    rerender(<PriceChart pairAddress={pairB} />)
    await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(pairB, '1h'))
  })

  it('uses defaultInterval for initial query when provided', async () => {
    renderWithProviders(<PriceChart pairAddress={pairA} defaultInterval="4h" />)
    await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(pairA, '4h'))
  })

  it('maps candle JSON into numeric OHLC points on the candlestick series', async () => {
    vi.mocked(indexerClient.getCandles).mockResolvedValue([candle({ open: '1.5', close: '1.6' })])
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => {
      const setData = lwChartTestDouble.seriesSpies[0]?.setData
      expect(setData).toHaveBeenCalled()
      const rows = setData?.mock.calls.at(-1)?.[0] as { open: number; close: number }[]
      expect(rows?.[0]).toMatchObject({ open: 1.5, close: 1.6 })
    })
  })

  it('calls setData again when the pair changes and new candles arrive', async () => {
    vi.mocked(indexerClient.getCandles).mockImplementation((addr: string) =>
      addr === pairA
        ? Promise.resolve([candle({ open: '1', close: '1.01' })])
        : Promise.resolve([candle({ open: '2', close: '2.02' })])
    )
    const { rerender } = renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() =>
      expect(lwChartTestDouble.seriesSpies[0]?.setData).toHaveBeenCalledWith(
        expect.arrayContaining([expect.objectContaining({ open: 1 })])
      )
    )
    rerender(<PriceChart pairAddress={pairB} />)
    await waitFor(() => {
      const spies = lwChartTestDouble.seriesSpies
      const candleSetData = spies.length >= 4 ? spies.at(-2)?.setData : spies[0]?.setData
      expect(candleSetData).toHaveBeenLastCalledWith(expect.arrayContaining([expect.objectContaining({ open: 2 })]))
    })
  })
})
