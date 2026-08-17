import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { createChart, CandlestickSeries, HistogramSeries, type IChartApi } from 'lightweight-charts'
import { applyChartDisplayInvert, type FactoryCandlePoint } from '../priceChartCandles'
import { usdCandlePriceFormatFromPoints } from '../priceChartPriceScale'
import { syncPriceChartIndicatorOverlays, type IndicatorSeriesRefs } from '../priceChartLightweightIndicatorSync'
import { syncCandleSeriesData } from '../priceChartLightweightSeriesSync'
import { minLowInVisibleLogicalRange } from '../priceChartPriceScale'
import {
  chartBundleFromCandles,
  chartBundleFromIndexerRows,
  indexerCandleRow,
  makeChartCandlePoints,
  volumePointsFromIndexer,
} from '@/test/chartTestFixtures'
import { baseRealChartOptions } from '@/test/chartTestOptions'
import {
  createRealChartWithUsdAutoscale,
  invokeUsdAutoscaleWithOriginal,
  mountChartContainer,
  setChartCssVars,
} from '@/test/chartRealLibraryHarness'

/** CI ceiling for large-candle real-library tests (GitLab #229). */
const LARGE_CANDLE_CI_MAX = 2000

describe('lightweight-charts (real module, Vitest #211)', () => {
  let container: HTMLDivElement | null = null

  beforeEach(() => {
    setChartCssVars()
  })

  afterEach(() => {
    container?.remove()
    container = null
  })

  it('imports the real lightweight-charts package (not vi.mock)', async () => {
    const { createChart: realCreate } = await import('lightweight-charts')
    expect(realCreate).toBeTypeOf('function')
    expect(String(realCreate)).not.toMatch(/mock/i)
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

describe('lightweight-charts large candle datasets (#229)', () => {
  let container: HTMLDivElement | null = null

  beforeEach(() => {
    setChartCssVars()
  })

  afterEach(() => {
    container?.remove()
    container = null
  })

  function assertLargeCandleInit(count: number) {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = chart.addSeries(CandlestickSeries, {})
    series.setData(makeChartCandlePoints(count))
    chart.timeScale().fitContent()
    expect(container.querySelectorAll('canvas').length).toBeGreaterThan(0)
    chart.remove()
  }

  it(`initializes ${500} candles within timeout`, () => {
    assertLargeCandleInit(500)
  })

  it(`initializes ${1500} candles within timeout`, { timeout: 25_000 }, () => {
    assertLargeCandleInit(1500)
  })

  it.runIf(Boolean(process.env.CI))(
    `CI soak: initializes ${LARGE_CANDLE_CI_MAX} candles (documented ceiling)`,
    { timeout: 30_000 },
    () => {
      assertLargeCandleInit(LARGE_CANDLE_CI_MAX)
    }
  )

  it('setData refresh 500→600 does not create a second chart', () => {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = chart.addSeries(CandlestickSeries, {})
    series.setData(makeChartCandlePoints(500))
    const canvasCountAfterFirst = container.querySelectorAll('canvas').length
    series.setData(makeChartCandlePoints(600))
    expect(container.querySelectorAll('canvas').length).toBe(canvasCountAfterFirst)
    chart.remove()
  })

  it('update on an older bar throws Cannot update oldest data', () => {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = chart.addSeries(CandlestickSeries, {})
    const previous = makeChartCandlePoints(4)
    series.setData(previous)
    const inverted = applyChartDisplayInvert(factoryFromUsd(previous, 86.48), true)
    expect(inverted[0]).toBeDefined()
    expect(() => series.update(inverted[0]!)).toThrow(/Cannot update oldest data/)
    chart.remove()
  })

  it('pair invert historical rewrite uses setData and does not throw (#524 / #543)', () => {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const series = chart.addSeries(CandlestickSeries, {})
    const previous = makeChartCandlePoints(8)
    series.setData(previous)
    const inverted = applyChartDisplayInvert(factoryFromUsd(previous, 86.48), true)
    expect(inverted.length).toBe(previous.length)
    expect(inverted[inverted.length - 1]!.close).toBeCloseTo(previous[previous.length - 1]!.close / 86.48, 6)
    expect(inverted[inverted.length - 1]!.close).not.toBeCloseTo(1 / previous[previous.length - 1]!.close, 3)
    expect(() => syncCandleSeriesData(series, previous, inverted)).not.toThrow()
    chart.remove()
  })

  it('inverted USTR-scale last close is ~0.012 not ~1 (GitLab #543 A1)', () => {
    container = mountChartContainer()
    const chart = createChart(container, baseRealChartOptions(640, 400))
    const usd = makeChartCandlePoints(3).map((p) => ({ ...p, open: 1.06, high: 1.08, low: 1.04, close: 1.06 }))
    const inverted = applyChartDisplayInvert(factoryFromUsd(usd, 86.48), true)
    const series = chart.addSeries(CandlestickSeries, {
      priceFormat: usdCandlePriceFormatFromPoints(inverted),
    })
    series.setData(inverted)
    expect(inverted[inverted.length - 1]!.close).toBeCloseTo(0.012258, 5)
    expect(usdCandlePriceFormatFromPoints(inverted).precision).toBeGreaterThan(2)
    chart.remove()
  })

  it('cLUNC-scale last-value format is not 2-dp 0.00 (GitLab #543 A4)', () => {
    const clunc = makeChartCandlePoints(2).map((p) => ({
      ...p,
      open: 0.000047,
      high: 0.000048,
      low: 0.000046,
      close: 0.000047,
    }))
    const fmt = usdCandlePriceFormatFromPoints(clunc)
    expect(fmt.precision).toBeGreaterThanOrEqual(6)
    expect(fmt.minMove).toBeLessThanOrEqual(1e-6)
    expect((0.000047).toFixed(2)).toBe('0.00')
    expect((0.000047).toFixed(fmt.precision)).not.toBe('0.00')
  })
})

function factoryFromUsd(points: ReturnType<typeof makeChartCandlePoints>, human: number): FactoryCandlePoint[] {
  return points.map((p) => ({
    time: p.time,
    usd: p,
    human: { time: p.time, open: human, high: human, low: human, close: human },
  }))
}

describe('USD autoscale with real visible logical range (#151, #229)', () => {
  let container: HTMLDivElement | null = null
  let chart: IChartApi | null = null

  beforeEach(() => {
    setChartCssVars()
  })

  afterEach(() => {
    chart?.remove()
    chart = null
    container?.remove()
    container = null
  })

  it('never returns minValue below zero when all candles are visible', () => {
    container = mountChartContainer()
    const points = makeChartCandlePoints(30)
    const wired = createRealChartWithUsdAutoscale(container, points)
    chart = wired.chart
    wired.series.setData(points)
    chart.timeScale().fitContent()

    const info = invokeUsdAutoscaleWithOriginal(wired.series, -5, 10)
    expect(info?.priceRange?.minValue).toBeGreaterThanOrEqual(0)
  })

  it('clamps to lowest visible candle low after zoom (#151)', () => {
    container = mountChartContainer()
    const points = makeChartCandlePoints(80)
    const wired = createRealChartWithUsdAutoscale(container, points)
    chart = wired.chart
    wired.series.setData(points)
    chart.timeScale().fitContent()

    const zoomFrom = 20
    const zoomTo = 40
    chart.timeScale().setVisibleLogicalRange({ from: zoomFrom, to: zoomTo })

    const logical = chart.timeScale().getVisibleLogicalRange()
    expect(logical).not.toBeNull()
    const expectedMinLow = minLowInVisibleLogicalRange(points, logical)
    expect(expectedMinLow).not.toBeNull()

    const info = invokeUsdAutoscaleWithOriginal(wired.series, -100, 200)
    expect(info?.priceRange?.minValue).toBeGreaterThanOrEqual(0)
    expect(info?.priceRange?.minValue).toBeGreaterThanOrEqual(expectedMinLow!)
  })

  it('single candle autoscale never goes below zero', () => {
    container = mountChartContainer()
    const points = makeChartCandlePoints(1)
    const wired = createRealChartWithUsdAutoscale(container, points)
    chart = wired.chart
    wired.series.setData(points)
    chart.timeScale().fitContent()

    const info = invokeUsdAutoscaleWithOriginal(wired.series, -1, 5)
    expect(info?.priceRange?.minValue).toBeGreaterThanOrEqual(0)
  })

  it('uses full-series min low when visible logical range is null', () => {
    container = mountChartContainer()
    const points = makeChartCandlePoints(10)
    const wired = createRealChartWithUsdAutoscale(container, points)
    chart = wired.chart
    wired.series.setData(points)

    const info = invokeUsdAutoscaleWithOriginal(wired.series, -50, 100)
    const fullMinLow = minLowInVisibleLogicalRange(points, null)
    expect(fullMinLow).not.toBeNull()
    expect(info?.priceRange?.minValue).toBeGreaterThanOrEqual(fullMinLow!)
  })
})
