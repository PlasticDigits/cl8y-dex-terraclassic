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

vi.mock('lightweight-charts', () => ({
  createChart: vi.fn(() => {
    const volumePane = { setHeight: vi.fn(), setStretchFactor: vi.fn() }
    return {
      remove: vi.fn(),
      addPane: vi.fn(() => volumePane),
      panes: vi.fn(() => [{ setHeight: vi.fn(), setStretchFactor: vi.fn() }, volumePane]),
      addSeries: vi.fn(() => {
        const setData = vi.fn()
        seriesSpies.push({ setData })
        return { setData }
      }),
      timeScale: () => ({ fitContent: vi.fn() }),
      applyOptions: vi.fn(),
      priceScale: vi.fn(() => ({ applyOptions: vi.fn() })),
    }
  }),
  CandlestickSeries: {},
  HistogramSeries: {},
}))
