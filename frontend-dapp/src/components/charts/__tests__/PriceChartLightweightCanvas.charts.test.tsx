import { describe, it, expect, beforeEach } from 'vitest'
import type { ComponentProps } from 'react'
import { render, screen, waitFor } from '@testing-library/react'
import { PriceChartLightweightCanvas } from '../PriceChartLightweightCanvas'
import { chartBundleFromCandles, makeChartCandlePoints } from '@/test/chartTestFixtures'

function renderCanvas(
  props: Partial<ComponentProps<typeof PriceChartLightweightCanvas>> = {},
  size = { width: 720, height: 420 }
) {
  const bundle = chartBundleFromCandles(props.candlePoints?.length ?? 24)
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

async function waitForRealCanvas() {
  await waitFor(
    () => {
      const root = screen.getByTestId('price-chart-lightweight-canvas')
      expect(root.querySelector('canvas')).toBeTruthy()
    },
    { timeout: 8000 }
  )
}

describe('PriceChartLightweightCanvas (real lightweight-charts, #211)', () => {
  beforeEach(() => {
    document.documentElement.style.setProperty('--color-positive', '#22c55e')
    document.documentElement.style.setProperty('--color-negative', '#ef4444')
    document.documentElement.style.setProperty('--focus-ring', '#38bdf8')
  })

  it('mounts real canvas for multi-candle fixtures', async () => {
    renderCanvas()
    await waitForRealCanvas()
  })

  it('mounts real canvas for a single candle', async () => {
    const single = makeChartCandlePoints(1)
    const bundle = chartBundleFromCandles(1)
    render(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={single}
          volumePoints={bundle.volumePoints}
          sma7Points={[]}
          sma25Points={[]}
          rsiPoints={[]}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )
    await waitForRealCanvas()
  })

  it('updates candle data via setData without remounting canvas', async () => {
    const bundleA = chartBundleFromCandles(10)
    const bundleB = chartBundleFromCandles(15)
    const { rerender } = render(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundleA.candlePoints}
          volumePoints={bundleA.volumePoints}
          sma7Points={bundleA.sma7Points}
          sma25Points={bundleA.sma25Points}
          rsiPoints={bundleA.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )
    await waitForRealCanvas()
    const root = screen.getByTestId('price-chart-lightweight-canvas')
    const canvasCount = root.querySelectorAll('canvas').length

    rerender(
      <div style={{ width: 640, height: 400 }}>
        <PriceChartLightweightCanvas
          candlePoints={bundleB.candlePoints}
          volumePoints={bundleB.volumePoints}
          sma7Points={bundleB.sma7Points}
          sma25Points={bundleB.sma25Points}
          rsiPoints={bundleB.rsiPoints}
          showSma7={false}
          showSma25={false}
          showRsi={false}
        />
      </div>
    )

    await waitFor(() => {
      expect(root.querySelectorAll('canvas').length).toBe(canvasCount)
    })
  })

  it('toggles MA7 overlay without unmounting canvas', async () => {
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
    await waitForRealCanvas()
    const root = screen.getByTestId('price-chart-lightweight-canvas')
    const before = root.querySelectorAll('canvas').length

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
      expect(root.querySelector('canvas')).toBeTruthy()
      expect(root.querySelectorAll('canvas').length).toBeGreaterThanOrEqual(before)
    })
  })

  it('unmount cleans up without throwing', async () => {
    const { unmount } = renderCanvas()
    await waitForRealCanvas()
    expect(() => unmount()).not.toThrow()
  })

  it('applies positive width and height after double requestAnimationFrame (GitLab #225)', async () => {
    const { container } = renderCanvas()
    await waitForRealCanvas()

    const chartRoot = container.querySelector('[data-testid="price-chart-lightweight-canvas"]') as HTMLElement
    expect(chartRoot.clientWidth).toBeGreaterThan(0)
    expect(chartRoot.clientHeight).toBeGreaterThanOrEqual(320)

    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => resolve())
      })
    })

    const canvases = chartRoot.querySelectorAll('canvas')
    expect(canvases.length).toBeGreaterThan(0)
    for (const canvas of canvases) {
      expect(canvas.width).toBeGreaterThan(0)
      expect(canvas.height).toBeGreaterThan(0)
    }
  })
})
