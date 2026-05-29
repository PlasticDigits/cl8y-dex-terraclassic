import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import type { ComponentProps } from 'react'
import { StrictMode } from 'react'
import { render, waitFor } from '@testing-library/react'
import { CandlestickSeries, createChart, HistogramSeries, LineSeries } from 'lightweight-charts'
import { PriceChartLightweightCanvas } from '../PriceChartLightweightCanvas'
import { chartBundleFromCandles, makeChartCandlePoints } from '@/test/chartTestFixtures'
import { lwChartTestDouble } from '@/test/lightweightChartsJsdomMock'

function setElementClientSize(el: HTMLElement, width: number, height: number) {
  Object.defineProperty(el, 'clientWidth', { configurable: true, get: () => width })
  Object.defineProperty(el, 'clientHeight', { configurable: true, get: () => height })
}

async function flushDoubleRaf() {
  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve())
    })
  })
}

function renderCanvas(
  props: Partial<ComponentProps<typeof PriceChartLightweightCanvas>> = {},
  containerSize?: { width: number; height: number }
) {
  const bundle = chartBundleFromCandles(props.candlePoints?.length ?? 24)
  const size = containerSize ?? { width: 640, height: 400 }
  const result = render(
    <div style={{ width: size.width, height: size.height }}>
      <PriceChartLightweightCanvas
        candlePoints={bundle.candlePoints}
        volumePoints={bundle.volumePoints}
        sma7Points={bundle.sma7Points}
        sma25Points={bundle.sma25Points}
        rsiPoints={bundle.rsiPoints}
        showSma7={false}
        showSma25={false}
        showRsi={false}
        {...props}
      />
    </div>
  )
  const chartRoot = result.container.querySelector(
    '[data-testid="price-chart-lightweight-canvas"]'
  ) as HTMLElement | null
  if (chartRoot) setElementClientSize(chartRoot, size.width, size.height)
  return result
}

describe('PriceChartLightweightCanvas lifecycle (stub, GitLab #225)', () => {
  beforeEach(() => {
    lwChartTestDouble.reset()
    document.documentElement.style.setProperty('--color-positive', '#22c55e')
    document.documentElement.style.setProperty('--color-negative', '#ef4444')
  })

  afterEach(() => {
    lwChartTestDouble.reset()
  })

  it('applyOptions runs with positive dimensions after double requestAnimationFrame', async () => {
    const { container } = renderCanvas({}, { width: 640, height: 400 })
    await waitFor(() => expect(vi.mocked(createChart)).toHaveBeenCalled())

    const chartRoot = container.querySelector('[data-testid="price-chart-lightweight-canvas"]') as HTMLElement
    setElementClientSize(chartRoot, 720, 480)

    const chart = lwChartTestDouble.lastChart()!
    vi.mocked(chart.applyOptions).mockClear()

    await flushDoubleRaf()

    await waitFor(() => {
      expect(chart.applyOptions).toHaveBeenCalled()
      const lastCall = vi.mocked(chart.applyOptions).mock.calls.at(-1)?.[0] as
        | { width?: number; height?: number }
        | undefined
      expect(lastCall?.width).toBeGreaterThan(0)
      expect(lastCall?.height).toBeGreaterThanOrEqual(320)
    })
  })

  it('ResizeObserver resize updates chart height via applyOptions', async () => {
    const { container } = renderCanvas()
    await waitFor(() => expect(lwChartTestDouble.lastChart()).toBeDefined())

    const chartRoot = container.querySelector('[data-testid="price-chart-lightweight-canvas"]') as HTMLElement
    setElementClientSize(chartRoot, 640, 500)

    const chart = lwChartTestDouble.lastChart()!
    vi.mocked(chart.applyOptions).mockClear()

    lwChartTestDouble.fireResize()

    await waitFor(() => {
      const lastCall = vi.mocked(chart.applyOptions).mock.calls.at(-1)?.[0] as { height?: number } | undefined
      expect(lastCall?.height).toBeGreaterThanOrEqual(320)
    })
  })

  it('unmount before dynamic import completes does not throw or create a chart', async () => {
    lwChartTestDouble.blockNextImport()
    const { unmount } = renderCanvas()

    unmount()
    expect(() => lwChartTestDouble.releaseBlockedImport()).not.toThrow()

    await waitFor(() => {
      expect(vi.mocked(createChart)).not.toHaveBeenCalled()
      expect(lwChartTestDouble.chartInstances).toHaveLength(0)
    })
  })

  it('React StrictMode double init leaves a single live chart after import resolves', async () => {
    lwChartTestDouble.blockNextImport()
    const bundle = chartBundleFromCandles(12)

    render(
      <StrictMode>
        <div style={{ width: 640, height: 400 }}>
          <PriceChartLightweightCanvas
            candlePoints={bundle.candlePoints}
            volumePoints={bundle.volumePoints}
            sma7Points={bundle.sma7Points}
            sma25Points={bundle.sma25Points}
            rsiPoints={bundle.rsiPoints}
            showSma7={false}
            showSma25={false}
            showRsi={false}
          />
        </div>
      </StrictMode>
    )

    lwChartTestDouble.releaseBlockedImport()

    await waitFor(() => expect(vi.mocked(createChart)).toHaveBeenCalledTimes(1))
    expect(lwChartTestDouble.chartInstances).toHaveLength(1)
  })

  it('unmount after init calls chart.remove exactly once', async () => {
    const { unmount } = renderCanvas()
    await waitFor(() => expect(lwChartTestDouble.lastChart()).toBeDefined())

    const chart = lwChartTestDouble.lastChart()!
    unmount()

    expect(chart.remove).toHaveBeenCalledTimes(1)
  })

  it('indicator toggles before chartModelReady do not throw; MA7 syncs after ready', async () => {
    lwChartTestDouble.blockNextImport()
    const bundle = chartBundleFromCandles(25)

    const { rerender } = render(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )

    expect(() => {
      rerender(
        <div style={{ width: 640, height: 400 }}>
          <PriceChartLightweightCanvas
            candlePoints={bundle.candlePoints}
            volumePoints={bundle.volumePoints}
            sma7Points={bundle.sma7Points}
            sma25Points={bundle.sma25Points}
            rsiPoints={bundle.rsiPoints}
            showSma7={true}
            showSma25={false}
            showRsi={false}
          />
        </div>
      )
    }).not.toThrow()

    lwChartTestDouble.releaseBlockedImport()

    await waitFor(() => {
      const chart = lwChartTestDouble.lastChart()
      expect(chart).toBeDefined()
      expect(chart!.addSeries).toHaveBeenCalled()
    })
  })

  it('RSI toggle before chartModelReady applies pane after init', async () => {
    lwChartTestDouble.blockNextImport()
    const bundle = chartBundleFromCandles(25)

    const { rerender } = render(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={true}
        />
      </div>
    )

    lwChartTestDouble.releaseBlockedImport()

    await waitFor(() => {
      const chart = lwChartTestDouble.lastChart()
      expect(chart?.addPane).toHaveBeenCalled()
    })

    rerender(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )

    await waitFor(() => {
      const chart = lwChartTestDouble.lastChart()
      expect(chart?.removePane).toHaveBeenCalledWith(2)
    })
  })

  it('rapid mount/unmount cycles do not throw', async () => {
    for (let i = 0; i < 10; i++) {
      const { unmount } = renderCanvas()
      await waitFor(() => expect(lwChartTestDouble.lastChart()).toBeDefined())
      expect(() => unmount()).not.toThrow()
      lwChartTestDouble.reset()
      vi.mocked(createChart).mockClear()
    }
  })
})

describe('PriceChartLightweightCanvas createChart contract (stub, GitLab #227)', () => {
  beforeEach(() => {
    lwChartTestDouble.reset()
    document.documentElement.style.setProperty('--color-positive', '#22c55e')
    document.documentElement.style.setProperty('--color-negative', '#ef4444')
  })

  afterEach(() => {
    lwChartTestDouble.reset()
  })

  it('mount passes layout, crosshair, and pane resize options to createChart', async () => {
    renderCanvas({}, { width: 640, height: 400 })
    await waitFor(() => expect(vi.mocked(createChart)).toHaveBeenCalledTimes(1))

    const opts = lwChartTestDouble.getLastCreateChartOptions() as {
      layout?: { panes?: { enableResize?: boolean } }
      crosshair?: { mode?: number }
      width?: number
      height?: number
    }
    expect(opts.layout?.attributionLogo).toBe(false)
    expect(opts.layout?.panes?.enableResize).toBe(false)
    expect(opts.crosshair?.mode).toBe(0)
    expect(opts.width).toBe(640)
    expect(opts.height).toBeGreaterThanOrEqual(320)
  })

  it('adds candlestick on pane 0 and volume histogram on pane 1', async () => {
    renderCanvas()
    await waitFor(() => expect(lwChartTestDouble.addSeriesCalls.length).toBeGreaterThanOrEqual(2))

    const candle = lwChartTestDouble.addSeriesCalls.find((c) => c.seriesType === CandlestickSeries)
    const volume = lwChartTestDouble.addSeriesCalls.find((c) => c.seriesType === HistogramSeries)
    expect(candle?.paneIndex).toBe(0)
    expect(volume?.paneIndex).toBe(1)
    expect(candle?.options).toMatchObject({ autoscaleInfoProvider: expect.any(Function) })
  })

  it('autoscaleInfoProvider clamps negative minValue via visible candle lows', async () => {
    renderCanvas()
    await waitFor(() => expect(lwChartTestDouble.getCandlestickAutoscaleProvider()).not.toBeNull())

    const provider = lwChartTestDouble.getCandlestickAutoscaleProvider()!
    const result = provider(() => ({
      priceRange: { minValue: -5, maxValue: 10 },
    }))
    expect(result?.priceRange?.minValue).toBeGreaterThanOrEqual(0)
  })

  it('autoscaleInfoProvider still returns when original() throws', async () => {
    renderCanvas()
    await waitFor(() => expect(lwChartTestDouble.getCandlestickAutoscaleProvider()).not.toBeNull())

    const provider = lwChartTestDouble.getCandlestickAutoscaleProvider()!
    expect(() =>
      provider(() => {
        throw new Error('original autoscale failed')
      })
    ).toThrow('original autoscale failed')
  })

  it('applyOptions records width and height from container after layout', async () => {
    const { container } = renderCanvas({}, { width: 800, height: 450 })
    await waitFor(() => expect(lwChartTestDouble.applyOptionsCalls.length).toBeGreaterThan(0))

    const chartRoot = container.querySelector('[data-testid="price-chart-lightweight-canvas"]') as HTMLElement
    setElementClientSize(chartRoot, 900, 500)
    lwChartTestDouble.fireResize()
    await flushDoubleRaf()

    await waitFor(() => {
      const last = lwChartTestDouble.getLastApplyOptions() as { width?: number; height?: number } | undefined
      expect(last?.width).toBe(900)
      expect(last?.height).toBeGreaterThanOrEqual(320)
    })
  })

  it('toggle MA7 adds line series on pane 0 and removeSeries when turned off', async () => {
    const bundle = chartBundleFromCandles(20)
    const { rerender } = render(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )
    await waitFor(() => expect(lwChartTestDouble.lastChart()).toBeDefined())

    const chart = lwChartTestDouble.lastChart()!
    vi.mocked(chart.addSeries).mockClear()
    vi.mocked(chart.removeSeries).mockClear()

    rerender(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={true}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )

    await waitFor(() => {
      const maCall = lwChartTestDouble.addSeriesCalls.find(
        (c) => c.seriesType === LineSeries && c.options.title === 'MA 7'
      )
      expect(maCall?.paneIndex).toBe(0)
    })

    rerender(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )

    await waitFor(() => expect(chart.removeSeries).toHaveBeenCalled())
  })

  it('toggle RSI adds pane and removePane(2) when turned off', async () => {
    const bundle = chartBundleFromCandles(20)
    const { rerender } = renderCanvas({ showRsi: true })
    await waitFor(() => expect(lwChartTestDouble.lastChart()?.addPane).toHaveBeenCalled())

    const chart = lwChartTestDouble.lastChart()!
    vi.mocked(chart.removePane).mockClear()

    rerender(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )

    await waitFor(() => expect(chart.removePane).toHaveBeenCalledWith(2))
  })

  it('prop change updates setData without a second createChart', async () => {
    const bundle = chartBundleFromCandles(12)
    const nextCandles = makeChartCandlePoints(12)
    const { rerender } = render(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundle.candlePoints}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )
    await waitFor(() => expect(lwChartTestDouble.seriesSpies.length).toBeGreaterThanOrEqual(2))

    const candleSetData = lwChartTestDouble.seriesSpies[0]!.setData
    vi.mocked(createChart).mockClear()

    rerender(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={nextCandles}
          volumePoints={bundle.volumePoints}
          sma7Points={bundle.sma7Points}
          sma25Points={bundle.sma25Points}
          rsiPoints={bundle.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )

    await waitFor(() => expect(candleSetData).toHaveBeenCalledWith(nextCandles))
    expect(vi.mocked(createChart)).not.toHaveBeenCalled()
  })

  it('reset clears contract spies between tests', () => {
    lwChartTestDouble.applyOptionsCalls.push({ options: { width: 1 } })
    lwChartTestDouble.reset()
    expect(lwChartTestDouble.applyOptionsCalls).toHaveLength(0)
    expect(lwChartTestDouble.addSeriesCalls).toHaveLength(0)
    expect(lwChartTestDouble.getCandlestickAutoscaleProvider()).toBeNull()
  })
})
