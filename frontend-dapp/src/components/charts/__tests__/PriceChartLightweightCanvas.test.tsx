import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import type { ComponentProps } from 'react'
import { StrictMode } from 'react'
import { render, waitFor } from '@testing-library/react'
import { createChart } from 'lightweight-charts'
import { PriceChartLightweightCanvas } from '../PriceChartLightweightCanvas'
import { chartBundleFromCandles } from '@/test/chartTestFixtures'
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
  return render(
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
