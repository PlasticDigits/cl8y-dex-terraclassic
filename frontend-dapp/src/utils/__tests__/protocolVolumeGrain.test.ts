import { describe, it, expect } from 'vitest'
import {
  clampProtocolVolumeLimit,
  formatPeriodAxisLabel,
  isAllowlistedProtocolVolumeLimit,
  isProtocolVolumeGrain,
  limitFromPlotWidth,
  maxPricedUsd,
  pointPeriod,
  PROTOCOL_VOLUME_GRAIN_MAX,
  PROTOCOL_VOLUME_GRAIN_MIN,
  sparseTimeLabelIndexes,
  usdAxisTicks,
} from '../protocolVolumeGrain'

describe('protocolVolumeGrain (GitLab #668)', () => {
  it('allowlists grain and rejects injection', () => {
    expect(isProtocolVolumeGrain('hourly')).toBe(true)
    expect(isProtocolVolumeGrain('daily')).toBe(true)
    expect(isProtocolVolumeGrain('monthly')).toBe(true)
    expect(isProtocolVolumeGrain('week')).toBe(false)
    expect(isProtocolVolumeGrain("daily' OR 1=1")).toBe(false)
    expect(isProtocolVolumeGrain('daily;')).toBe(false)
  })

  it('clamps limit to grain min/max', () => {
    expect(clampProtocolVolumeLimit('daily', 1)).toBe(PROTOCOL_VOLUME_GRAIN_MIN.daily)
    expect(clampProtocolVolumeLimit('daily', 999)).toBe(PROTOCOL_VOLUME_GRAIN_MAX.daily)
    expect(clampProtocolVolumeLimit('hourly', 200)).toBe(168)
    expect(clampProtocolVolumeLimit('monthly', 3)).toBe(6)
    expect(clampProtocolVolumeLimit('daily', Number.POSITIVE_INFINITY)).toBe(PROTOCOL_VOLUME_GRAIN_MIN.daily)
  })

  it('derives phone vs desktop bar counts from width', () => {
    const phone = limitFromPlotWidth(320, 'daily')
    const wide = limitFromPlotWidth(1280, 'daily')
    expect(phone).toBeLessThan(30)
    expect(phone).toBeGreaterThanOrEqual(PROTOCOL_VOLUME_GRAIN_MIN.daily)
    expect(wide).toBeGreaterThan(phone)
    expect(wide).toBeLessThanOrEqual(PROTOCOL_VOLUME_GRAIN_MAX.daily)
    expect(limitFromPlotWidth(1280, 'hourly')).toBeLessThanOrEqual(168)
    expect(limitFromPlotWidth(1280, 'monthly')).toBe(24)
  })

  it('rejects non-allowlisted limits before fetch', () => {
    expect(isAllowlistedProtocolVolumeLimit('daily', 14)).toBe(true)
    expect(isAllowlistedProtocolVolumeLimit('daily', 91)).toBe(false)
    expect(isAllowlistedProtocolVolumeLimit('hourly', 0)).toBe(false)
    expect(isAllowlistedProtocolVolumeLimit('hourly', 1.5)).toBe(false)
  })

  it('usd ticks include $0 and avoid divide-by-zero', () => {
    expect(usdAxisTicks(0)).toEqual([0])
    expect(usdAxisTicks(-1)).toEqual([0])
    expect(usdAxisTicks(Number.NaN)).toEqual([0])
    expect(usdAxisTicks(100)).toEqual([0, 25, 50, 75, 100])
  })

  it('sparse X labels do not mark every hourly bar', () => {
    expect(sparseTimeLabelIndexes(4)).toEqual([0, 1, 2, 3])
    expect(sparseTimeLabelIndexes(168).length).toBeLessThanOrEqual(5)
    expect(sparseTimeLabelIndexes(168)[0]).toBe(0)
    expect(sparseTimeLabelIndexes(168).at(-1)).toBe(167)
  })

  it('formats period keys and peak without NaN', () => {
    expect(formatPeriodAxisLabel('2026-08-26T14', 'hourly')).toBe('14')
    expect(formatPeriodAxisLabel('2026-08-26', 'daily')).toBe('08-26')
    expect(formatPeriodAxisLabel('2026-08', 'monthly')).toBe('2026-08')
    expect(pointPeriod({ utc_hour: '2026-08-26T14', volume_usd: '1', trade_count: 1 }, 'hourly')).toBe('2026-08-26T14')
    expect(maxPricedUsd([{ volume_usd: '10' }, { volume_usd: null }, { volume_usd: 'Infinity' }])).toBe(10)
  })
})
