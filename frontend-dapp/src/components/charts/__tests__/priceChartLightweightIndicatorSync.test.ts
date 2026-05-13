import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { syncPriceChartIndicatorOverlays, type IndicatorSeriesRefs } from '../priceChartLightweightIndicatorSync'

function mockLc() {
  return {
    LineSeries: {},
  } as unknown as typeof import('lightweight-charts')
}

function mockChart() {
  const panes = [{ setHeight: vi.fn() }, { setHeight: vi.fn() }]
  const removeSeries = vi.fn()
  const removePane = vi.fn((i: number) => {
    panes.splice(i, 1)
  })
  const addPane = vi.fn(() => {
    panes.push({ setHeight: vi.fn() })
    return panes[panes.length - 1]
  })
  const addSeries = vi.fn(() => ({
    setData: vi.fn(),
    priceScale: () => ({
      setAutoScale: vi.fn(),
      setVisibleRange: vi.fn(),
    }),
    createPriceLine: vi.fn(),
  }))
  return {
    panes: vi.fn(() => panes),
    addPane,
    removePane,
    addSeries,
    removeSeries,
  }
}

describe('syncPriceChartIndicatorOverlays', () => {
  beforeEach(() => {
    vi.spyOn(window, 'getComputedStyle').mockReturnValue({
      getPropertyValue: () => '#38bdf8',
    } as unknown as CSSStyleDeclaration)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('adds MA line series on pane 0 when enabled', () => {
    const chart = mockChart()
    const refs: IndicatorSeriesRefs = { sma7: null, sma25: null, rsi: null }
    syncPriceChartIndicatorOverlays(chart as never, mockLc(), refs, {
      showSma7: true,
      showSma25: false,
      showRsi: false,
      sma7Points: [{ time: 1 as never, value: 1.5 }],
      sma25Points: [],
      rsiPoints: [],
      chartHeightPx: 400,
    })
    expect(chart.addSeries).toHaveBeenCalledWith(
      {},
      expect.objectContaining({ title: 'MA 7' }),
      0
    )
    expect(refs.sma7).not.toBeNull()
  })

  it('removes MA series when disabled', () => {
    const chart = mockChart()
    const lineApi = { id: 'line' } as never
    const refs: IndicatorSeriesRefs = { sma7: lineApi, sma25: null, rsi: null }
    syncPriceChartIndicatorOverlays(chart as never, mockLc(), refs, {
      showSma7: false,
      showSma25: false,
      showRsi: false,
      sma7Points: [],
      sma25Points: [],
      rsiPoints: [],
      chartHeightPx: 400,
    })
    expect(chart.removeSeries).toHaveBeenCalledWith(lineApi)
    expect(refs.sma7).toBeNull()
  })

  it('adds RSI pane and series when enabled', () => {
    const chart = mockChart()
    const refs: IndicatorSeriesRefs = { sma7: null, sma25: null, rsi: null }
    syncPriceChartIndicatorOverlays(chart as never, mockLc(), refs, {
      showSma7: false,
      showSma25: false,
      showRsi: true,
      sma7Points: [],
      sma25Points: [],
      rsiPoints: [{ time: 1 as never, value: 55 }],
      chartHeightPx: 400,
    })
    expect(chart.addPane).toHaveBeenCalled()
    expect(chart.addSeries).toHaveBeenCalledWith(
      {},
      expect.objectContaining({ title: 'RSI 14' }),
      2
    )
    expect(refs.rsi).not.toBeNull()
  })

  it('removes RSI series and pane 2 when disabled', () => {
    const chart = mockChart()
    chart.addPane()
    const rsiApi = { id: 'rsi' } as never
    const refs: IndicatorSeriesRefs = { sma7: null, sma25: null, rsi: rsiApi }
    syncPriceChartIndicatorOverlays(chart as never, mockLc(), refs, {
      showSma7: false,
      showSma25: false,
      showRsi: false,
      sma7Points: [],
      sma25Points: [],
      rsiPoints: [],
      chartHeightPx: 400,
    })
    expect(chart.removeSeries).toHaveBeenCalledWith(rsiApi)
    expect(chart.removePane).toHaveBeenCalledWith(2)
    expect(refs.rsi).toBeNull()
  })
})
