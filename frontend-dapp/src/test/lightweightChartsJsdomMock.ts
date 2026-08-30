import { vi } from 'vitest'

type SeriesSpy = {
  setData: ReturnType<typeof vi.fn>
  update: ReturnType<typeof vi.fn>
  applyOptions: ReturnType<typeof vi.fn>
}

export type LwAddSeriesCall = {
  seriesType: unknown
  options: Record<string, unknown>
  paneIndex?: number
}

export type LwApplyOptionsCall = {
  options: Record<string, unknown>
}

export type LwChartMock = {
  remove: ReturnType<typeof vi.fn>
  applyOptions: ReturnType<typeof vi.fn>
  addPane: ReturnType<typeof vi.fn>
  removePane: ReturnType<typeof vi.fn>
  panes: ReturnType<typeof vi.fn>
  removeSeries: ReturnType<typeof vi.fn>
  addSeries: ReturnType<typeof vi.fn>
  timeScale: () => {
    fitContent: ReturnType<typeof vi.fn>
    getVisibleLogicalRange: ReturnType<typeof vi.fn>
  }
  priceScale: ReturnType<typeof vi.fn>
}

const {
  seriesSpies,
  chartInstances,
  addSeriesCalls,
  applyOptionsCalls,
  createChartOptionCalls,
  autoscaleProviderHolder,
  lightweightChartsModule,
  resetChartMockState,
} = vi.hoisted(() => {
  const seriesSpies: SeriesSpy[] = []
  const chartInstances: LwChartMock[] = []
  const addSeriesCalls: LwAddSeriesCall[] = []
  const applyOptionsCalls: LwApplyOptionsCall[] = []
  const createChartOptionCalls: Record<string, unknown>[] = []
  const autoscaleProviderHolder: {
    provider:
      | ((original: () => { priceRange?: { minValue: number; maxValue: number } } | null) => {
          priceRange?: { minValue: number; maxValue: number }
        } | null)
      | null
  } = { provider: null }

  const makePane = () => ({ setHeight: vi.fn(), setStretchFactor: vi.fn() })

  const createChart = vi.fn((_container: HTMLElement, options?: Record<string, unknown>) => {
    if (options) createChartOptionCalls.push(options)
    const panes: ReturnType<typeof makePane>[] = [makePane()]

    const timeScaleApi = {
      fitContent: vi.fn(),
      getVisibleLogicalRange: vi.fn(() => ({ from: 0, to: 10 })),
    }

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
      addSeries: vi.fn((seriesType: unknown, options?: Record<string, unknown>, paneIndex?: number) => {
        addSeriesCalls.push({
          seriesType,
          options: options ?? {},
          paneIndex,
        })
        if (options && typeof options.autoscaleInfoProvider === 'function') {
          autoscaleProviderHolder.provider = options.autoscaleInfoProvider as typeof autoscaleProviderHolder.provider
        }
        const setData = vi.fn()
        const update = vi.fn()
        const applyOptions = vi.fn()
        seriesSpies.push({ setData, update, applyOptions })
        return {
          setData,
          update,
          applyOptions,
          priceScale: () => ({
            setAutoScale: vi.fn(),
            setVisibleRange: vi.fn(),
            applyOptions: vi.fn(),
          }),
          createPriceLine: vi.fn(),
        }
      }),
      timeScale: () => timeScaleApi,
      applyOptions: vi.fn((options?: Record<string, unknown>) => {
        if (options) applyOptionsCalls.push({ options })
      }),
      priceScale: vi.fn(() => ({ applyOptions: vi.fn() })),
    }
    chartInstances.push(chart)
    return chart
  })

  const lightweightChartsModule = {
    createChart,
    CandlestickSeries: { __kind: 'CandlestickSeries' },
    HistogramSeries: { __kind: 'HistogramSeries' },
    LineSeries: { __kind: 'LineSeries' },
  }

  function resetChartMockState() {
    seriesSpies.length = 0
    chartInstances.length = 0
    addSeriesCalls.length = 0
    applyOptionsCalls.length = 0
    createChartOptionCalls.length = 0
    autoscaleProviderHolder.provider = null
    createChart.mockClear()
  }

  return {
    seriesSpies,
    chartInstances,
    addSeriesCalls,
    applyOptionsCalls,
    createChartOptionCalls,
    autoscaleProviderHolder,
    lightweightChartsModule,
    resetChartMockState,
  }
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
 * Contract surface for canvas options: GitLab #227; lifecycle: #225.
 */
export const lwChartTestDouble = {
  get seriesSpies(): SeriesSpy[] {
    return seriesSpies
  },
  get chartInstances(): LwChartMock[] {
    return chartInstances
  },
  get addSeriesCalls(): LwAddSeriesCall[] {
    return addSeriesCalls
  },
  get applyOptionsCalls(): LwApplyOptionsCall[] {
    return applyOptionsCalls
  },
  get createChartOptionCalls(): Record<string, unknown>[] {
    return createChartOptionCalls
  },
  lastChart(): LwChartMock | undefined {
    return chartInstances[chartInstances.length - 1]
  },
  getLastCreateChartOptions(): Record<string, unknown> | undefined {
    return createChartOptionCalls[createChartOptionCalls.length - 1]
  },
  getLastApplyOptions(): Record<string, unknown> | undefined {
    return applyOptionsCalls[applyOptionsCalls.length - 1]?.options
  },
  getCandlestickAutoscaleProvider():
    | ((original: () => { priceRange?: { minValue: number; maxValue: number } } | null) => {
        priceRange?: { minValue: number; maxValue: number }
      } | null)
    | null {
    return autoscaleProviderHolder.provider
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
