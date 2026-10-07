import { describe, it, expect } from 'vitest'
import {
  clampProtocolVolumeLimit,
  estimatedAxisLabelWidthPx,
  PROTOCOL_VOLUME_AXIS_LABEL_GAP_PX,
  formatPeriodAxisLabel,
  formatPeriodTooltip,
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

describe('protocolVolumeGrain (GitLab #668 / #677 / #703)', () => {
  it('formats the actual UTC period as a readable date without quality jargon', () => {
    expect(formatPeriodTooltip('2026-10-01', 'daily')).toBe('Oct 1, 2026')
    expect(formatPeriodTooltip('2026-10', 'monthly')).toBe('Oct 2026')
    expect(formatPeriodTooltip('2026-10-01T23', 'hourly')).toBe('Oct 1, 2026, 11:00 PM')
    expect(formatPeriodTooltip('2026-02-31', 'daily')).toBe('2026-02-31')
    expect(formatPeriodTooltip('<script>', 'daily')).toBe('<script>')
  })

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

  it('caps phone monthly at ≤12 so YY-MM ticks fit (GitLab #703)', () => {
    const phone = limitFromPlotWidth(320, 'monthly')
    expect(phone).toBeLessThanOrEqual(12)
    expect(phone).toBeGreaterThanOrEqual(PROTOCOL_VOLUME_GRAIN_MIN.monthly)
    expect(Number.isInteger(phone)).toBe(true)
    expect(limitFromPlotWidth(260, 'monthly')).toBe(PROTOCOL_VOLUME_GRAIN_MIN.monthly)
    expect(limitFromPlotWidth(1280, 'monthly')).toBe(24)
    expect(limitFromPlotWidth(728, 'monthly')).toBe(24)
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

  it('spaces all grains by their estimated width plus a readable gap at phone and desktop sizes', () => {
    for (const viewportWidth of [390, 2048]) {
      // Include narrow panel widths and wide viewports; production uses measured width.
      for (const plotWidth of [260, viewportWidth - 60]) {
        for (const grain of ['hourly', 'daily', 'monthly'] as const) {
          for (let count = 2; count <= PROTOCOL_VOLUME_GRAIN_MAX[grain]; count++) {
            const indexes = timeLabelIndexes(count, grain, plotWidth)
            expect(indexes[0]).toBe(0)
            expect(indexes.at(-1)).toBe(count - 1)
            const requiredGap = estimatedAxisLabelWidthPx(grain) + PROTOCOL_VOLUME_AXIS_LABEL_GAP_PX
            for (let i = 1; i < indexes.length; i++) {
              expect(((indexes[i] - indexes[i - 1]) * plotWidth) / count).toBeGreaterThanOrEqual(requiredGap)
            }
          }
        }
      }
    }
  })

  it('removes a colliding penultimate label without losing the first or last observation', () => {
    expect(timeLabelIndexes(22, 'daily', 272)).toEqual([0, 3, 6, 9, 12, 15, 18, 21])
    expect(timeLabelIndexes(23, 'daily', 272)).toEqual([0, 3, 6, 9, 12, 15, 18, 22])
    expect(timeLabelIndexes(6, 'monthly', 130)).toEqual([0, 2, 5])
    expect(timeLabelIndexes(1, 'daily', 272)).toEqual([0])
    expect(timeLabelIndexes(0, 'daily', 272)).toEqual([])
  })

  it('shows more labels when space permits without an arbitrary five-label ceiling', () => {
    expect(timeLabelIndexes(7, 'daily')).toEqual([0, 1, 2, 3, 4, 5, 6])
    expect(timeLabelStep(7, 'daily')).toBe(1)
    for (const grain of ['hourly', 'daily', 'monthly'] as const) {
      const count = PROTOCOL_VOLUME_GRAIN_MAX[grain]
      expect(timeLabelIndexes(count, grain, 1988).length).toBeGreaterThan(5)
      expect(timeLabelStep(count, grain, 1988)).toBeLessThan(timeLabelStep(count, grain, 260))
    }
  })

  it('formats period keys and peak without NaN', () => {
    expect(formatPeriodAxisLabel('2026-08-26T14', 'hourly')).toBe('14')
    expect(formatPeriodAxisLabel('2026-08-26', 'daily')).toBe('08-26')
    expect(formatPeriodAxisLabel('2026-08', 'monthly')).toBe('26-08')
    expect(formatPeriodAxisLabel('26-08', 'monthly')).toBe('26-08')
    expect(formatPeriodAxisLabel('', 'monthly')).toBe('—')
    expect(formatPeriodAxisLabel('<script>alert(1)</script>', 'monthly')).toBe('<scri')
    expect(formatPeriodAxisLabel('javascript:', 'monthly')).toBe('javas')
    expect(pointPeriod({ utc_hour: '2026-08-26T14', volume_usd: '1', trade_count: 1 }, 'hourly')).toBe('2026-08-26T14')
    expect(pointPeriod({ utc_month: '2026-08', volume_usd: '1', trade_count: 1 }, 'monthly')).toBe('2026-08')
    expect(maxPricedUsd([{ volume_usd: '10' }, { volume_usd: null }, { volume_usd: 'Infinity' }])).toBe(10)
    expect(maxPricedUsd([{ liquidity_usd: '20', quality: 'fresh' }, { liquidity_usd: null }], 'liquidity')).toBe(20)
    expect(maxPricedUsd([{ fees_usd: '3' }, { fees_usd: '0' }], 'fees')).toBe(3)
  })
})
