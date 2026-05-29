import { vi } from 'vitest'

type SeriesSpy = { setData: ReturnType<typeof vi.fn> }

export type LwChartMock = {
  remove: ReturnType<typeof vi.fn>
  applyOptions: ReturnType<typeof vi.fn>
  addPane: ReturnType<typeof vi.fn>
  removePane: ReturnType<typeof vi.fn>
  panes: ReturnType<typeof vi.fn>
  removeSeries: ReturnType<typeof vi.fn>
  addSeries: ReturnType<typeof vi.fn>
  timeScale: () => { fitContent: ReturnType<typeof vi.fn> }
  priceScale: ReturnType<typeof vi.fn>
}

const { seriesSpies, chartInstances, lightweightChartsModule, resetChartMockState } = vi.hoisted(() => {
  const seriesSpies: SeriesSpy[] = []
  const chartInstances: LwChartMock[] = []

  const makePane = () => ({ setHeight: vi.fn(), setStretchFactor: vi.fn() })

  const createChart = vi.fn(() => {
    const panes: ReturnType<typeof makePane>[] = [makePane()]

    const chart: LwChartMock = {
      remove: vi.fn(),
      addPane: vi.fn(() => {
        panes.push(makePane())
        return panes[panes.length - 1]!
      }),
      removePane: vi.fn((index: number) => {
        panes.splice(index, 1)
      }),
      panes: vi.fn(() => panes),
      removeSeries: vi.fn(),
      addSeries: vi.fn(() => {
        const setData = vi.fn()
        seriesSpies.push({ setData })
        return {
          setData,
          priceScale: () => ({
            setAutoScale: vi.fn(),
            setVisibleRange: vi.fn(),
            applyOptions: vi.fn(),
          }),
          createPriceLine: vi.fn(),
        }
      }),
      timeScale: () => ({ fitContent: vi.fn() }),
      applyOptions: vi.fn(),
      priceScale: vi.fn(() => ({ applyOptions: vi.fn() })),
    }
    chartInstances.push(chart)
    return chart
  })

  const lightweightChartsModule = {
    createChart,
    CandlestickSeries: {},
    HistogramSeries: {},
    LineSeries: {},
  }

  function resetChartMockState() {
    seriesSpies.length = 0
    chartInstances.length = 0
    createChart.mockClear()
  }

  return { seriesSpies, chartInstances, lightweightChartsModule, resetChartMockState }
})

let deferredImport: Promise<typeof import('lightweight-charts')> | null = null
let releaseDeferredImport: (() => void) | null = null

type ResizeObserverEntry = {
  callback: ResizeObserverCallback
  targets: Set<Element>
}

const activeResizeObservers: ResizeObserverEntry[] = []

function installTrackingResizeObserver() {
  if (typeof window === 'undefined') return

  window.ResizeObserver = class ResizeObserver {
    private readonly entry: ResizeObserverEntry

    constructor(callback: ResizeObserverCallback) {
      this.entry = { callback, targets: new Set() }
      activeResizeObservers.push(this.entry)
    }

    observe(target: Element) {
      this.entry.targets.add(target)
    }

    unobserve(target: Element) {
      this.entry.targets.delete(target)
    }

    disconnect() {
      this.entry.targets.clear()
      const idx = activeResizeObservers.indexOf(this.entry)
      if (idx >= 0) activeResizeObservers.splice(idx, 1)
    }
  } as unknown as typeof ResizeObserver
}

installTrackingResizeObserver()

/**
 * lightweight-charts expects Canvas + layout; jsdom provides neither. Stub the module so
 * unit/integration Vitest runs stay deterministic (real library runs in the browser / E2E).
 */
export const lwChartTestDouble = {
  get seriesSpies(): SeriesSpy[] {
    return seriesSpies
  },
  get chartInstances(): LwChartMock[] {
    return chartInstances
  },
  lastChart(): LwChartMock | undefined {
    return chartInstances[chartInstances.length - 1]
  },
  /** Hold the next `loadPriceChartLightweightModule()` until `releaseBlockedImport()`. */
  blockNextImport() {
    deferredImport = new Promise((resolve) => {
      releaseDeferredImport = () => resolve(lightweightChartsModule)
    })
  },
  releaseBlockedImport() {
    releaseDeferredImport?.()
    releaseDeferredImport = null
    deferredImport = null
  },
  /** Invoke callbacks for all live ResizeObserver instances (lifecycle tests, #225). */
  fireResize() {
    for (const entry of [...activeResizeObservers]) {
      if (entry.targets.size === 0) continue
      entry.callback([], {} as ResizeObserver)
    }
  },
  reset() {
    resetChartMockState()
    activeResizeObservers.length = 0
    this.releaseBlockedImport()
  },
}

vi.mock('lightweight-charts', () => lightweightChartsModule)

vi.mock('@/components/charts/priceChartLightweightModule', () => ({
  loadPriceChartLightweightModule: vi.fn(async () => {
    if (deferredImport) return deferredImport
    return lightweightChartsModule
  }),
}))
