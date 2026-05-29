import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { createChart, CandlestickSeries, HistogramSeries, type AutoscaleInfo, type IChartApi } from 'lightweight-charts'
import { syncPriceChartIndicatorOverlays, type IndicatorSeriesRefs } from '../priceChartLightweightIndicatorSync'
import { clampUsdPriceChartAutoscale, minLowInVisibleLogicalRange } from '../priceChartPriceScale'
import {
  chartBundleFromCandles,
  chartBundleFromIndexerRows,
  indexerCandleRow,
  makeChartCandlePoints,
  volumePointsFromIndexer,
} from '@/test/chartTestFixtures'
import { baseRealChartOptions } from '@/test/chartTestOptions'

function mountChartContainer(width = 640, height = 400): HTMLDivElement {
  const el = document.createElement('div')
  el.style.width = `${width}px`
  el.style.height = `${height}px`
  document.body.appendChild(el)
  Object.defineProperty(el, 'clientWidth', { configurable: true, value: width })
  Object.defineProperty(el, 'clientHeight', { configurable: true, value: height })
  return el
}

function wireUsdAutoscale(chart: IChartApi, candlePoints: ReturnType<typeof makeChartCandlePoints>) {
  const series = chart.addSeries(CandlestickSeries, {
    autoscaleInfoProvider: (original: () => AutoscaleInfo | null) => {
      const raw = original()
      const logical = chart.timeScale().getVisibleLogicalRange()
      const visibleMinLow = minLowInVisibleLogicalRange(candlePoints, logical)
      return clampUsdPriceChartAutoscale(raw, visibleMinLow)
    },
  })
  return series
}

function setChartCssVars() {
  document.documentElement.style.setProperty('--color-positive', '#22c55e')
  document.documentElement.style.setProperty('--color-negative', '#ef4444')
  document.documentElement.style.setProperty('--focus-ring', '#38bdf8')
}

describe('lightweight-charts (real module, Vitest #211)', () => {
  let container: HTMLDivElement | null = null

  beforeEach(() => {
    setChartCssVars()
  })

  afterEach(() => {
    container?.remove()
    container = null
  })

  it('imports the real lightweight-charts package (not vi.mock)', () => {
    expect(createChart).toBeTypeOf('function')
    expect(String(createChart)).not.toMatch(/mock/i)
  })

  it('createChart mounts with a single candle', () => {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = chart.addSeries(CandlestickSeries, {})
    series.setData(makeChartCandlePoints(1))
    expect(container.querySelector('canvas')).toBeTruthy()
    chart.remove()
  })

  it('createChart mounts with many candles without throwing', () => {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = chart.addSeries(CandlestickSeries, {})
    series.setData(makeChartCandlePoints(220))
    chart.timeScale().fitContent()
    expect(container.querySelectorAll('canvas').length).toBeGreaterThan(0)
    chart.remove()
  })

  it('setData updates series without a second createChart', () => {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = chart.addSeries(CandlestickSeries, {})
    series.setData(makeChartCandlePoints(5))
    const canvasCountAfterFirst = container.querySelectorAll('canvas').length
    series.setData(makeChartCandlePoints(8))
    expect(container.querySelectorAll('canvas').length).toBe(canvasCountAfterFirst)
    chart.remove()
  })

  it('USD autoscale provider never returns minValue below zero', () => {
    container = mountChartContainer()
    const points = makeChartCandlePoints(30)
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = wireUsdAutoscale(chart, points)
    series.setData(points)
    chart.timeScale().fitContent()

    const provider = (
      series as unknown as {
        options: () => { autoscaleInfoProvider?: (o: () => AutoscaleInfo | null) => AutoscaleInfo | null }
      }
    ).options?.()?.autoscaleInfoProvider

    expect(provider).toBeTypeOf('function')
    const info = provider!(() => ({
      priceRange: { minValue: -5, maxValue: 10 },
    }))
    expect(info?.priceRange?.minValue).toBeGreaterThanOrEqual(0)
    chart.remove()
  })

  it('volume histogram uses base when quote volume is zero', () => {
    const rows = [
      indexerCandleRow({ volume_quote: '0', volume_base: '42' }),
      indexerCandleRow({
        open_time: '2024-01-01T13:00:00.000Z',
        volume_quote: '0',
        volume_base: '99',
      }),
    ]
    const volume = volumePointsFromIndexer(rows)
    expect(volume[0]?.value).toBe(42)
    expect(volume[1]?.value).toBe(99)

    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    chart.addSeries(CandlestickSeries, {}).setData(chartBundleFromIndexerRows(rows).candlePoints)
    chart.addPane()
    const hist = chart.addSeries(HistogramSeries, { priceFormat: { type: 'volume' } }, 1)
    hist.setData(volume)
    expect(chart.panes().length).toBe(2)
    chart.remove()
  })

  it('syncPriceChartIndicatorOverlays adds MA line on pane 0 and removes it', async () => {
    const lc = await import('lightweight-charts')
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const bundle = chartBundleFromCandles(25)
    chart.addSeries(CandlestickSeries, {}).setData(bundle.candlePoints)
    chart.addPane()

    const refs: IndicatorSeriesRefs = { sma7: null, sma25: null, rsi: null }
    syncPriceChartIndicatorOverlays(chart, lc, refs, {
      showSma7: true,
      showSma25: false,
      showRsi: false,
      ...bundle,
      chartHeightPx: 400,
    })
    expect(refs.sma7).not.toBeNull()

    syncPriceChartIndicatorOverlays(chart, lc, refs, {
      showSma7: false,
      showSma25: false,
      showRsi: false,
      ...bundle,
      chartHeightPx: 400,
    })
    expect(refs.sma7).toBeNull()
    chart.remove()
  })

  it('syncPriceChartIndicatorOverlays adds RSI pane and removePane(2) on disable', async () => {
    const lc = await import('lightweight-charts')
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const bundle = chartBundleFromCandles(25)
    chart.addSeries(CandlestickSeries, {}).setData(bundle.candlePoints)
    chart.addPane()
    chart.addSeries(HistogramSeries, { priceFormat: { type: 'volume' } }, 1).setData(bundle.volumePoints)

    const refs: IndicatorSeriesRefs = { sma7: null, sma25: null, rsi: null }
    syncPriceChartIndicatorOverlays(chart, lc, refs, {
      showSma7: false,
      showSma25: false,
      showRsi: true,
      ...bundle,
      chartHeightPx: 400,
    })
    expect(chart.panes().length).toBe(3)
    expect(refs.rsi).not.toBeNull()

    syncPriceChartIndicatorOverlays(chart, lc, refs, {
      showSma7: false,
      showSma25: false,
      showRsi: false,
      ...bundle,
      chartHeightPx: 400,
    })
    expect(refs.rsi).toBeNull()
    expect(chart.panes().length).toBe(2)
    chart.remove()
  })

  it('chart.remove runs on teardown without leaking canvases', () => {
    container = mountChartContainer()
    for (let i = 0; i < 5; i++) {
      const chart = createChart(container, baseRealChartOptions(640, 400))
      chart.addSeries(CandlestickSeries, {}).setData(makeChartCandlePoints(3))
      chart.remove()
    }
    expect(container.querySelectorAll('canvas').length).toBe(0)
  })
})
