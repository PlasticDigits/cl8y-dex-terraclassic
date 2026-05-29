import {
  createChart,
  CandlestickSeries,
  type AutoscaleInfo,
  type IChartApi,
  type ISeriesApi,
} from 'lightweight-charts'
import type { ChartCandlePoint } from '@/components/charts/priceChartCandles'
import { clampUsdPriceChartAutoscale, minLowInVisibleLogicalRange } from '@/components/charts/priceChartPriceScale'
import { baseRealChartOptions } from '@/test/chartTestOptions'

/** DOM container sized for real lightweight-charts Vitest runs (GitLab #211, #229). */
export function mountChartContainer(width = 640, height = 400): HTMLDivElement {
  const el = document.createElement('div')
  el.style.width = `${width}px`
  el.style.height = `${height}px`
  document.body.appendChild(el)
  Object.defineProperty(el, 'clientWidth', { configurable: true, value: width })
  Object.defineProperty(el, 'clientHeight', { configurable: true, value: height })
  return el
}

export function setChartCssVars() {
  document.documentElement.style.setProperty('--color-positive', '#22c55e')
  document.documentElement.style.setProperty('--color-negative', '#ef4444')
  document.documentElement.style.setProperty('--focus-ring', '#38bdf8')
}

/** Mirrors production `PriceChartLightweightCanvas` USD autoscale wiring ([#151], [#229]). */
export function wireUsdAutoscale(chart: IChartApi, candlePoints: ChartCandlePoint[]): ISeriesApi<'Candlestick'> {
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

export function createRealChartWithUsdAutoscale(
  container: HTMLDivElement,
  candlePoints: ChartCandlePoint[],
  width = 640,
  height = 400
): { chart: IChartApi; series: ISeriesApi<'Candlestick'> } {
  const chart = createChart(container, baseRealChartOptions(width, height))
  const series = wireUsdAutoscale(chart, candlePoints)
  return { chart, series }
}

type AutoscaleProvider = (original: () => AutoscaleInfo | null) => AutoscaleInfo | null

export function readCandleAutoscaleProvider(series: ISeriesApi<'Candlestick'>): AutoscaleProvider {
  const provider = (
    series as unknown as {
      options: () => { autoscaleInfoProvider?: AutoscaleProvider }
    }
  ).options?.()?.autoscaleInfoProvider
  if (typeof provider !== 'function') {
    throw new Error('candlestick series missing autoscaleInfoProvider')
  }
  return provider
}

/** Invokes the series provider with a synthetic `original()` negative floor (regression #151). */
export function invokeUsdAutoscaleWithOriginal(
  series: ISeriesApi<'Candlestick'>,
  originalMin: number,
  originalMax: number
): AutoscaleInfo | null {
  const provider = readCandleAutoscaleProvider(series)
  return provider(() => ({
    priceRange: { minValue: originalMin, maxValue: originalMax },
  }))
}
