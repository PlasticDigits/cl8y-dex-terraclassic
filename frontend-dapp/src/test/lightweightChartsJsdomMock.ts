import { vi } from 'vitest'

type SeriesSpy = { setData: ReturnType<typeof vi.fn> }

const seriesSpies: SeriesSpy[] = []

/**
 * lightweight-charts expects Canvas + layout; jsdom provides neither. Stub the module so
 * unit/integration Vitest runs stay deterministic (real library runs in the browser / E2E).
 */
export const lwChartTestDouble = {
  get seriesSpies(): SeriesSpy[] {
    return seriesSpies
  },
  reset() {
    seriesSpies.length = 0
  },
}

vi.mock('lightweight-charts', () => {
  const makePane = () => ({ setHeight: vi.fn(), setStretchFactor: vi.fn() })

  return {
    createChart: vi.fn(() => {
      const panes: ReturnType<typeof makePane>[] = [makePane()]

      const chart = {
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
      return chart
    }),
    CandlestickSeries: {},
    HistogramSeries: {},
    LineSeries: {},
  }
})
