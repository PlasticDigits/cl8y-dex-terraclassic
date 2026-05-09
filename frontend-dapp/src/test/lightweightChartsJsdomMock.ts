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
  const volumePane = { setHeight: vi.fn(), setStretchFactor: vi.fn() }
  const rsiPane = { setHeight: vi.fn(), setStretchFactor: vi.fn() }
  return {
    createChart: vi.fn(() => ({
      remove: vi.fn(),
      addPane: vi.fn(() => volumePane),
      panes: vi.fn(() => [{ setHeight: vi.fn(), setStretchFactor: vi.fn() }, volumePane, rsiPane]),
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
    })),
    CandlestickSeries: {},
    HistogramSeries: {},
    LineSeries: {},
  }
})
