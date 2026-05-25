import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import PriceChart from '../PriceChart'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerCandle } from '@/types'
import { createChart } from 'lightweight-charts'
import { lwChartTestDouble } from '@/test/lightweightChartsJsdomMock'
import { formatNum } from '@/utils/formatAmount'

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
    lwChartTestDouble.reset()
    vi.mocked(createChart).mockClear()
    vi.mocked(indexerClient.getCandles).mockReset()
    vi.mocked(indexerClient.getCandles).mockResolvedValue([candle()])
    vi.mocked(indexerClient.getPairStats).mockReset()
    vi.mocked(indexerClient.getPairStats).mockResolvedValue({ ...emptyStats })
  })

  it('shows headline last price from tape when tapeLastPriceUsd is provided', async () => {
    renderWithProviders(<PriceChart pairAddress={pairA} tapeLastPriceUsd="3.14159265" />)
    await waitFor(() => expect(screen.queryByText(/loading chart/i)).not.toBeInTheDocument())
    expect(screen.getByTestId('trade-chart-headline-price')).toHaveTextContent(/Last/)
    expect(screen.getByTestId('trade-chart-headline-price')).toHaveTextContent(formatNum('3.14159265', 6))
  })

  it('shows headline from last candle close when tape is omitted', async () => {
    vi.mocked(indexerClient.getCandles).mockResolvedValue([
      candle({ open_time: '2024-01-02T12:00:00.000Z', open: '2', close: '2.5' }),
    ])
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(screen.getByTestId('trade-chart-headline-price')).toBeInTheDocument())
    expect(screen.getByTestId('trade-chart-headline-price')).toHaveTextContent(formatNum(2.5, 6))
  })

  it('renders chart toolbar with indicators menu and fullscreen control', async () => {
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(screen.queryByText(/loading chart/i)).not.toBeInTheDocument())
    expect(screen.getByTestId('price-chart-indicators-trigger')).toBeInTheDocument()
    expect(screen.getByTestId('price-chart-fullscreen')).toBeInTheDocument()
    expect(screen.getByText(/volume \(quote, else base\)/i)).toBeInTheDocument()
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

  it('reuses one chart instance across many interval switches (GitLab #148)', async () => {
    const user = userEvent.setup()
    const intervalClose: Record<string, string> = {
      '1m': '1.01',
      '5m': '1.05',
      '15m': '1.15',
      '1h': '1.6',
      '4h': '1.64',
      '1d': '1.7',
      '1w': '1.77',
    }
    vi.mocked(indexerClient.getCandles).mockImplementation((_addr, iv) =>
      Promise.resolve([candle({ close: intervalClose[iv] ?? '9' })])
    )
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(screen.getByTestId('price-chart-lightweight-canvas')).toBeInTheDocument())
    expect(vi.mocked(createChart)).toHaveBeenCalledTimes(1)

    const sequence = ['1m', '5m', '15m', '1h', '4h', '1d', '1w', '1m', '5m', '15m', '1h'] as const
    for (const iv of sequence) {
      await user.click(screen.getByRole('button', { name: iv }))
      await waitFor(() => expect(indexerClient.getCandles).toHaveBeenCalledWith(pairA, iv))
    }

    expect(vi.mocked(createChart)).toHaveBeenCalledTimes(1)
    await waitFor(() => {
      const rows = lwChartTestDouble.seriesSpies[0]?.setData?.mock.calls.at(-1)?.[0] as { close: number }[]
      expect(rows?.[0]?.close).toBe(1.6)
    })
    expect(screen.queryByText(/loading chart/i)).not.toBeInTheDocument()
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

  it('adds a line series when MA 7 is toggled on after the chart loads', async () => {
    const user = userEvent.setup()
    const candles = Array.from({ length: 20 }, (_, i) =>
      candle({
        open_time: new Date(Date.UTC(2024, 0, 1, i, 0, 0)).toISOString(),
        open: String(1 + i * 0.01),
        close: String(1 + i * 0.01 + 0.002),
      })
    )
    vi.mocked(indexerClient.getCandles).mockResolvedValue(candles)
    renderWithProviders(<PriceChart pairAddress={pairA} />)
    await waitFor(() => expect(lwChartTestDouble.seriesSpies.length).toBeGreaterThanOrEqual(2))
    await user.click(screen.getByTestId('price-chart-indicators-trigger'))
    await user.click(screen.getByRole('checkbox', { name: /ma 7/i }))
    await waitFor(() => expect(lwChartTestDouble.seriesSpies.length).toBe(3))
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
