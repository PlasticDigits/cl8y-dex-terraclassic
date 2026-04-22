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
    const setData = vi.fn()
    seriesSpies.push({ setData })
    return {
      remove: vi.fn(),
      addSeries: vi.fn(() => ({ setData })),
      timeScale: () => ({ fitContent: vi.fn() }),
      applyOptions: vi.fn(),
    }
  }),
  CandlestickSeries: {},
}))
