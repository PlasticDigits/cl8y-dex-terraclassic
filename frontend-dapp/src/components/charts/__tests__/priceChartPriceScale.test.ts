import { describe, expect, it } from 'vitest'
import type { LogicalRange } from 'lightweight-charts'
import { clampUsdPriceChartAutoscale, minLowInVisibleLogicalRange } from '../priceChartPriceScale'
import type { ChartCandlePoint } from '../priceChartCandles'

const lr = (from: number, to: number): LogicalRange => ({ from, to }) as LogicalRange

const pts = (values: number[]): ChartCandlePoint[] =>
  values.map((low, i) => ({
    time: (1000 + i) as ChartCandlePoint['time'],
    open: low,
    high: low + 2,
    low,
    close: low + 1,
  }))

describe('minLowInVisibleLogicalRange', () => {
  it('returns null for empty points', () => {
    expect(minLowInVisibleLogicalRange([], lr(0, 1))).toBeNull()
  })

  it('uses full series when logical range is null', () => {
    const p = pts([100, 50, 75])
    expect(minLowInVisibleLogicalRange(p, null)).toBe(50)
  })

  it('restricts to visible logical indices', () => {
    const p = pts([10, 20, 30, 40])
    expect(minLowInVisibleLogicalRange(p, lr(1, 2))).toBe(20)
  })

  it('handles fractional logical edges', () => {
    const p = pts([100, 90, 80])
    expect(minLowInVisibleLogicalRange(p, lr(0.2, 1.7))).toBe(90)
  })
})

describe('clampUsdPriceChartAutoscale', () => {
  it('lifts a padded minimum that dipped below zero up to the visible floor', () => {
    const res = clampUsdPriceChartAutoscale({ priceRange: { minValue: -25, maxValue: 120 } }, 88)
    expect(res?.priceRange?.minValue).toBe(88)
    expect(res?.priceRange?.maxValue).toBe(120)
  })

  it('uses 0 when visible min low is unknown', () => {
    const res = clampUsdPriceChartAutoscale({ priceRange: { minValue: -10, maxValue: 50 } }, null)
    expect(res?.priceRange?.minValue).toBe(0)
  })

  it('does not push the minimum above the default autoscale when already valid', () => {
    const res = clampUsdPriceChartAutoscale({ priceRange: { minValue: 85, maxValue: 110 } }, 90)
    expect(res?.priceRange?.minValue).toBe(90)
  })

  it('passes through null autoscale', () => {
    expect(clampUsdPriceChartAutoscale(null, 1)).toBeNull()
  })

  it('passes through when priceRange is null', () => {
    expect(clampUsdPriceChartAutoscale({ priceRange: null }, 1)).toEqual({ priceRange: null })
  })

  it('floors a negative visible min at 0 (bad data guard)', () => {
    const res = clampUsdPriceChartAutoscale({ priceRange: { minValue: -5, maxValue: 10 } }, -3)
    expect(res?.priceRange?.minValue).toBe(0)
  })
})
