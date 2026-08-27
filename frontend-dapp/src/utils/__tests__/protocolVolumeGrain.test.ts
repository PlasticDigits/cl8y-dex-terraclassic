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
  timeLabelIndexes,
  timeLabelStep,
  usdAxisTicks,
} from '../protocolVolumeGrain'

describe('protocolVolumeGrain (GitLab #668 / #677)', () => {
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

  it('labels every daily/monthly bar or every second (GitLab #677)', () => {
    expect(timeLabelIndexes(7, 'daily')).toEqual([0, 1, 2, 3, 4, 5, 6])
    expect(timeLabelStep(7, 'daily')).toBe(1)
    const daily90 = timeLabelIndexes(90, 'daily')
    expect(timeLabelStep(90, 'daily')).toBeLessThanOrEqual(2)
    expect(daily90[0]).toBe(0)
    expect(daily90.at(-1)).toBe(89)
    expect(daily90.length).toBeGreaterThanOrEqual(Math.ceil(90 / 2))
    expect(daily90.length).toBeLessThanOrEqual(90)

    const monthly6 = timeLabelIndexes(6, 'monthly')
    expect(monthly6[0]).toBe(0)
    expect(monthly6.at(-1)).toBe(5)
    expect(timeLabelStep(6, 'monthly')).toBeLessThanOrEqual(2)
    expect(monthly6.length).toBeGreaterThanOrEqual(Math.ceil(6 / 2))

    const monthly24 = timeLabelIndexes(24, 'monthly')
    expect(timeLabelStep(24, 'monthly')).toBeLessThanOrEqual(2)
    expect(monthly24[0]).toBe(0)
    expect(monthly24.at(-1)).toBe(23)
    expect(monthly24.length).toBeGreaterThanOrEqual(Math.ceil(24 / 2))
  })

  it('hourly 12 is all or every 2nd; 168 is width-clamped, not capped at 5', () => {
    const h12 = timeLabelIndexes(12, 'hourly')
    expect(h12[0]).toBe(0)
    expect(h12.at(-1)).toBe(11)
    expect(timeLabelStep(12, 'hourly')).toBeLessThanOrEqual(2)
    expect(h12.length).toBeGreaterThanOrEqual(Math.ceil(12 / 2))

    const h168 = timeLabelIndexes(168, 'hourly')
    expect(h168[0]).toBe(0)
    expect(h168.at(-1)).toBe(167)
    expect(h168.length).toBeGreaterThan(5)
    expect(h168.length).toBeLessThanOrEqual(168)
    expect(timeLabelStep(168, 'hourly')).toBeGreaterThan(2)
  })

  it('formats period keys and peak without NaN', () => {
    expect(formatPeriodAxisLabel('2026-08-26T14', 'hourly')).toBe('14')
    expect(formatPeriodAxisLabel('2026-08-26', 'daily')).toBe('08-26')
    expect(formatPeriodAxisLabel('2026-08', 'monthly')).toBe('2026-08')
    expect(pointPeriod({ utc_hour: '2026-08-26T14', volume_usd: '1', trade_count: 1 }, 'hourly')).toBe('2026-08-26T14')
    expect(maxPricedUsd([{ volume_usd: '10' }, { volume_usd: null }, { volume_usd: 'Infinity' }])).toBe(10)
    expect(maxPricedUsd([{ liquidity_usd: '20' }, { liquidity_usd: null }], 'liquidity')).toBe(20)
    expect(maxPricedUsd([{ fees_usd: '3' }, { fees_usd: '0' }], 'fees')).toBe(3)
  })
})
