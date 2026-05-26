import { describe, it, expect } from 'vitest'
import { keepPreviousCandlesForIntervalSwitch } from '../priceChartCandlesPlaceholder'

describe('keepPreviousCandlesForIntervalSwitch', () => {
  const candles = [{ open_time: '2024-01-01T00:00:00.000Z', open: '1', close: '1' }]

  it('keeps previous data when only the interval changes', () => {
    const result = keepPreviousCandlesForIntervalSwitch('terra1pairA', candles, {
      queryKey: ['candles', 'terra1pairA', '1h'],
    })
    expect(result).toBe(candles)
  })

  it('drops previous data when the pair address changes', () => {
    const result = keepPreviousCandlesForIntervalSwitch('terra1pairB', candles, {
      queryKey: ['candles', 'terra1pairA', '1h'],
    })
    expect(result).toBeUndefined()
  })

  it('returns undefined when there is no previous query', () => {
    expect(keepPreviousCandlesForIntervalSwitch('terra1pairA', candles, undefined)).toBeUndefined()
  })
})
