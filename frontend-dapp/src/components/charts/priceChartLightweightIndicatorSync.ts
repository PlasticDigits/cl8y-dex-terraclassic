import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import type { IndicatorLinePoint } from './priceChartIndicators'
import { rsiPaneHeightPx } from './priceChartPaneHeights'

type LcModule = typeof import('lightweight-charts')

export interface IndicatorSeriesRefs {
  sma7: ISeriesApi<'Line'> | null
  sma25: ISeriesApi<'Line'> | null
  rsi: ISeriesApi<'Line'> | null
}

/** Attach or detach MA / RSI overlays without recreating the chart (avoids async init races on toggle). */
export function syncPriceChartIndicatorOverlays(
  chart: IChartApi,
  lc: LcModule,
  refs: IndicatorSeriesRefs,
  opts: {
    showSma7: boolean
    showSma25: boolean
    showRsi: boolean
    sma7Points: IndicatorLinePoint[]
    sma25Points: IndicatorLinePoint[]
    rsiPoints: IndicatorLinePoint[]
    chartHeightPx: number
  }
): void {
  const { LineSeries } = lc
  const h = Math.max(320, opts.chartHeightPx)

  if (opts.showSma7) {
    if (!refs.sma7) {
      refs.sma7 = chart.addSeries(
        LineSeries,
        {
          color: '#38bdf8',
          lineWidth: 2,
          title: 'MA 7',
          priceLineVisible: false,
          lastValueVisible: true,
        },
        0
      )
      refs.sma7.setData(opts.sma7Points)
    }
  } else if (refs.sma7) {
    chart.removeSeries(refs.sma7)
    refs.sma7 = null
  }

  if (opts.showSma25) {
    if (!refs.sma25) {
      refs.sma25 = chart.addSeries(
        LineSeries,
        {
          color: '#fbbf24',
          lineWidth: 2,
          title: 'MA 25',
          priceLineVisible: false,
          lastValueVisible: true,
        },
        0
      )
      refs.sma25.setData(opts.sma25Points)
    }
  } else if (refs.sma25) {
    chart.removeSeries(refs.sma25)
    refs.sma25 = null
  }

  if (opts.showRsi) {
    if (!refs.rsi) {
      chart.addPane()
      const rsiPaneIndex = chart.panes().length - 1
      chart.panes()[rsiPaneIndex]?.setHeight(rsiPaneHeightPx(h))

      const accent = getComputedStyle(document.documentElement).getPropertyValue('--focus-ring').trim() || '#38bdf8'
      refs.rsi = chart.addSeries(
        LineSeries,
        {
          color: accent || '#a78bfa',
          lineWidth: 2,
          title: 'RSI 14',
          priceLineVisible: false,
          lastValueVisible: true,
          priceFormat: { type: 'price', precision: 1, minMove: 0.1 },
        },
        rsiPaneIndex
      )
      refs.rsi.setData(opts.rsiPoints)
      refs.rsi.priceScale().setAutoScale(false)
      refs.rsi.priceScale().setVisibleRange({ from: 0, to: 100 })
      refs.rsi.createPriceLine({
        price: 70,
        color: 'rgba(248, 113, 113, 0.55)',
        lineWidth: 1,
        lineStyle: 2,
        axisLabelVisible: true,
        title: '70',
      })
      refs.rsi.createPriceLine({
        price: 30,
        color: 'rgba(74, 222, 128, 0.55)',
        lineWidth: 1,
        lineStyle: 2,
        axisLabelVisible: true,
        title: '30',
      })
    }
  } else if (refs.rsi) {
    chart.removeSeries(refs.rsi)
    refs.rsi = null
    if (chart.panes().length > 2) {
      chart.removePane(2)
    }
  }
}
