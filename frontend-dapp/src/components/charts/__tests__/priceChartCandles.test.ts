import { describe, it, expect } from 'vitest'
import {
  applyChartDisplayInvert,
  candleOpenTimeSeconds,
  indexerCandlesToChartPoints,
  indexerCandlesToFactoryPoints,
  indexerCandlesToVolumeHistogramPoints,
  parseChartFiniteNumber,
} from '../priceChartCandles'
import type { IndexerCandle } from '@/types'
import { invertOhlc, invertUsdNumber } from '@/utils/tradePairDisplayOrientation'

function row(overrides: Partial<IndexerCandle> = {}): IndexerCandle {
  return {
    open_time: '2024-01-01T12:00:00.000Z',
    open: '1',
    high: '1.1',
    low: '0.9',
    close: '1.05',
    open_human: '86.48',
    high_human: '90',
    low_human: '80',
    close_human: '86.48',
    volume_base: '100',
    volume_quote: '105',
    trade_count: 3,
    ...overrides,
  }
}

function assertAllFiniteOHLC(pts: ReturnType<typeof indexerCandlesToChartPoints>) {
  for (const p of pts) {
    expect(Number.isFinite(p.open)).toBe(true)
    expect(Number.isFinite(p.high)).toBe(true)
    expect(Number.isFinite(p.low)).toBe(true)
    expect(Number.isFinite(p.close)).toBe(true)
    expect(Number.isFinite(p.time as number)).toBe(true)
  }
}

describe('parseChartFiniteNumber', () => {
  it('returns finite parses and null for non-finite', () => {
    expect(parseChartFiniteNumber('1.5')).toBe(1.5)
    expect(parseChartFiniteNumber('abc')).toBeNull()
    expect(parseChartFiniteNumber('NaN')).toBeNull()
    expect(parseChartFiniteNumber('Infinity')).toBeNull()
    expect(parseChartFiniteNumber('1e309')).toBeNull()
    expect(parseChartFiniteNumber('')).toBeNull()
  })
})

describe('candleOpenTimeSeconds', () => {
  it('returns unix seconds for valid ISO times and null for garbage', () => {
    expect(candleOpenTimeSeconds('2024-01-01T12:00:00.000Z')).toBe(1704110400)
    expect(candleOpenTimeSeconds('not-a-date')).toBeNull()
    expect(candleOpenTimeSeconds('<script>alert(1)</script>')).toBeNull()
  })
})

describe('indexerCandlesToChartPoints', () => {
  it('returns empty array for undefined or empty input', () => {
    expect(indexerCandlesToChartPoints(undefined)).toEqual([])
    expect(indexerCandlesToChartPoints([])).toEqual([])
  })

  it('drops rows without open and close', () => {
    expect(indexerCandlesToChartPoints([row({ open: '', close: '' })])).toEqual([])
  })

  it('sorts by time ascending', () => {
    const a = row({ open_time: '2024-01-02T12:00:00.000Z', open: '2', close: '2', high: '2', low: '2' })
    const b = row({ open_time: '2024-01-01T12:00:00.000Z', open: '1', close: '1', high: '1', low: '1' })
    const pts = indexerCandlesToChartPoints([a, b])
    expect(pts).toHaveLength(2)
    expect(pts[0].open).toBe(1)
    expect(pts[1].open).toBe(2)
    assertAllFiniteOHLC(pts)
  })

  it('maps newest-first JSON to non-decreasing time (#705)', () => {
    const newer = row({
      open_time: '2026-08-29T12:00:00.000Z',
      open: '3',
      close: '3',
      high: '3',
      low: '3',
    })
    const older = row({
      open_time: '2026-08-20T00:00:00.000Z',
      open: '1',
      close: '1',
      high: '1',
      low: '1',
    })
    const pts = indexerCandlesToChartPoints([newer, older])
    expect(pts).toHaveLength(2)
    expect(Number(pts[0].time)).toBeLessThan(Number(pts[1].time))
    assertAllFiniteOHLC(pts)
  })

  it('newest-N rows still invertUsd per bar, not 1/x (#705 / #543)', () => {
    const newer = row({
      open_time: '2026-08-29T12:00:00.000Z',
      open: '1.06',
      high: '1.08',
      low: '1.04',
      close: '1.06',
      open_human: '86.48',
      high_human: '88',
      low_human: '84',
      close_human: '86.48',
    })
    const older = row({
      open_time: '2026-08-20T00:00:00.000Z',
      open: '0.000047',
      high: '0.000048',
      low: '0.000046',
      close: '0.000047',
      open_human: '0.000047',
      high_human: '0.000048',
      low_human: '0.000046',
      close_human: '0.000047',
    })
    const factory = indexerCandlesToFactoryPoints([newer, older])
    const inv = applyChartDisplayInvert(factory, true)
    expect(inv).toHaveLength(2)
    expect(Number(inv[0].time)).toBeLessThan(Number(inv[1].time))
    expect(inv[0].close).toBeCloseTo(0.000047 / 0.000047, 8)
    expect(inv[1].close).toBeCloseTo(1.06 / 86.48, 8)
    expect(inv[1].close).not.toBeCloseTo(1 / 1.06, 3)
    expect(inv[1].close).toBeCloseTo(invertUsdNumber(1.06, 86.48)!, 10)
  })

  it('drops non-numeric open or close', () => {
    expect(indexerCandlesToChartPoints([row({ open: 'abc', close: '1' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ open: '1', close: 'xyz' })])).toEqual([])
  })

  it('drops NaN and Infinity string OHLC', () => {
    expect(indexerCandlesToChartPoints([row({ close: 'NaN' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ high: 'Infinity' })])).toEqual([])
  })

  it('drops extreme float strings', () => {
    expect(indexerCandlesToChartPoints([row({ open: '1e309', close: '1', high: '1', low: '1' })])).toEqual([])
  })

  it('drops rows with missing or invalid high/low', () => {
    expect(indexerCandlesToChartPoints([row({ high: '', low: '0.9' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ high: 'bad', low: '0.9' })])).toEqual([])
  })

  it('drops invalid open_time', () => {
    expect(indexerCandlesToChartPoints([row({ open_time: 'invalid' })])).toEqual([])
  })

  it('keeps only valid rows in a mixed batch', () => {
    const pts = indexerCandlesToChartPoints([
      row({ open: '1', close: '1', high: '1', low: '1' }),
      row({ open: 'bad', close: '1' }),
      row({ open: '3', close: '3', high: '3', low: '3', open_time: '2024-01-02T12:00:00.000Z' }),
    ])
    expect(pts).toHaveLength(2)
    assertAllFiniteOHLC(pts)
  })

  it('stable-sorts duplicate open_time without throwing', () => {
    const t = '2024-01-01T12:00:00.000Z'
    const pts = indexerCandlesToChartPoints([
      row({ open: '2', close: '2', high: '2', low: '2', open_time: t }),
      row({ open: '1', close: '1', high: '1', low: '1', open_time: t }),
    ])
    expect(pts).toHaveLength(2)
    expect(pts[0].time).toBe(pts[1].time)
    assertAllFiniteOHLC(pts)
  })

  it('drops runtime-weird open values without throwing', () => {
    const weird = row({ open: '1', close: '1' })
    ;(weird as { open: unknown }).open = { toString: () => '1' } as unknown as string
    expect(() => indexerCandlesToChartPoints([weird])).not.toThrow()
    assertAllFiniteOHLC(indexerCandlesToChartPoints([weird]))
  })
})

describe('indexerCandlesToVolumeHistogramPoints', () => {
  it('maps quote volume and colors from open vs close', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [
        row({ open: '1', close: '1.1', volume_quote: '50' }),
        row({
          open_time: '2024-01-02T12:00:00.000Z',
          open: '2',
          close: '1.5',
          high: '2',
          low: '1',
          volume_quote: '12',
        }),
      ],
      '#00ff00',
      '#ff0000'
    )
    expect(pts).toHaveLength(2)
    expect(pts[0].value).toBe(50)
    expect(pts[0].color).toBe('#00ff00')
    expect(pts[1].value).toBe(12)
    expect(pts[1].color).toBe('#ff0000')
  })

  it('falls back to base volume when quote is zero', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [row({ open: '1', close: '1.1', volume_quote: '0', volume_base: '42' })],
      '#00ff00',
      '#ff0000'
    )
    expect(pts[0]?.value).toBe(42)
  })

  it('applyChartDisplayInvert uses invertUsd not 1/x (GitLab #543 A1 / H2)', () => {
    const factory = indexerCandlesToFactoryPoints([
      row({
        open: '1.06',
        high: '1.08',
        low: '1.04',
        close: '1.06',
        open_human: '86.48',
        high_human: '88',
        low_human: '84',
        close_human: '86.48',
      }),
    ])
    const inv = applyChartDisplayInvert(factory, true)
    expect(inv).toHaveLength(1)
    expect(inv[0].open).toBeCloseTo(1.06 / 86.48, 8)
    expect(inv[0].close).toBeCloseTo(1.06 / 86.48, 8)
    expect(inv[0].open).toBeCloseTo(0.012258, 5)
    expect(inv[0].open).not.toBeCloseTo(1 / 1.06, 3)
    expect(inv[0].high).toBeGreaterThanOrEqual(inv[0].low)
    expect(inv[0].time).toBe(factory[0].time)
    expect(applyChartDisplayInvert(factory, false)).toEqual([factory[0].usd])
  })

  it('inverted cLUNC factory USD / human → ~$1 UST1, not 1/x (A5)', () => {
    const factory = indexerCandlesToFactoryPoints([
      row({
        open: '0.000047',
        high: '0.000048',
        low: '0.000046',
        close: '0.000047',
        open_human: '0.000047',
        high_human: '0.000048',
        low_human: '0.000046',
        close_human: '0.000047',
      }),
    ])
    const inv = applyChartDisplayInvert(factory, true)
    expect(inv[0]?.close).toBeCloseTo(1, 5)
    expect(inv[0]?.close).not.toBeCloseTo(1 / 0.000047, 0)
  })

  it('drops missing USD even when human is 21260 (H8)', () => {
    expect(
      indexerCandlesToFactoryPoints([
        row({
          open: '',
          close: '',
          open_human: '21260',
          high_human: '21260',
          low_human: '21260',
          close_human: '21260',
        }),
      ])
    ).toEqual([])
    expect(indexerCandlesToChartPoints([row({ open: '0', close: '21260', high: '21260', low: '1' })])).toEqual([])
  })

  it('drops invert when human is 0 / missing (H2)', () => {
    const zeroHuman = indexerCandlesToFactoryPoints([
      row({ open_human: '0', high_human: '86', low_human: '80', close_human: '86' }),
    ])
    expect(applyChartDisplayInvert(zeroHuman, true)).toEqual([])
    const noHuman = indexerCandlesToFactoryPoints([
      row({ open_human: null, high_human: null, low_human: null, close_human: null }),
    ])
    expect(noHuman[0]?.human).toBeNull()
    expect(applyChartDisplayInvert(noHuman, true)).toEqual([])
    expect(applyChartDisplayInvert(noHuman, false)).toHaveLength(1)
  })

  it('drops non-finite / non-positive USD (H1)', () => {
    expect(indexerCandlesToChartPoints([row({ open: '-1', close: '1' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ open: '0', close: '1' })])).toEqual([])
    expect(indexerCandlesToChartPoints([row({ close: 'Infinity' })])).toEqual([])
  })

  it('display last close matches invertUsd tape identity (A1 fixture)', () => {
    const factoryUsd = 1.06
    const human = 86.48
    const factory = indexerCandlesToFactoryPoints([
      row({
        open: String(factoryUsd),
        high: String(factoryUsd),
        low: String(factoryUsd),
        close: String(factoryUsd),
        open_human: String(human),
        high_human: String(human),
        low_human: String(human),
        close_human: String(human),
      }),
    ])
    const close = applyChartDisplayInvert(factory, true)[0]?.close
    expect(close).toBeDefined()
    expect(close).toBeCloseTo(invertUsdNumber(factoryUsd, human)!, 10)
    expect(invertOhlc([factory[0].usd])[0]?.close).toBeCloseTo(1 / factoryUsd, 8)
    expect(close).not.toBeCloseTo(1 / factoryUsd, 3)
  })

  it('keeps per-bar invertUsd when as-of quote USD varies (GitLab #568)', () => {
    const bars = indexerCandlesToFactoryPoints([
      row({
        open_time: '2024-01-01T00:00:00.000Z',
        open: '1.0',
        high: '1.0',
        low: '1.0',
        close: '1.0',
        open_human: '200',
        high_human: '200',
        low_human: '200',
        close_human: '200',
        trade_count: 1,
      }),
      row({
        open_time: '2024-01-02T00:00:00.000Z',
        open: '0.8',
        high: '0.8',
        low: '0.8',
        close: '0.8',
        open_human: '200',
        high_human: '200',
        low_human: '200',
        close_human: '200',
        trade_count: 0,
        volume_base: '0',
        volume_quote: '0',
      }),
    ])
    const inv = applyChartDisplayInvert(bars, true)
    expect(inv).toHaveLength(2)
    expect(inv[0].close).toBeCloseTo(0.005, 8)
    expect(inv[1].close).toBeCloseTo(0.004, 8)
    expect(new Set(inv.map((p) => p.close)).size).toBe(2)
  })

  it('does not scale history by the latest tape human (GitLab #543 / #568)', () => {
    const bars = indexerCandlesToFactoryPoints([
      row({
        open_time: '2024-01-01T00:00:00.000Z',
        open: '1.06',
        high: '1.06',
        low: '1.06',
        close: '1.06',
        open_human: '86.48',
        high_human: '86.48',
        low_human: '86.48',
        close_human: '86.48',
      }),
      row({
        open_time: '2024-01-02T00:00:00.000Z',
        open: '1.10',
        high: '1.10',
        low: '1.10',
        close: '1.10',
        open_human: '90',
        high_human: '90',
        low_human: '90',
        close_human: '90',
      }),
    ])
    const inv = applyChartDisplayInvert(bars, true)
    expect(inv[0].close).toBeCloseTo(1.06 / 86.48, 8)
    expect(inv[1].close).toBeCloseTo(1.1 / 90, 8)
    expect(inv[0].close).not.toBeCloseTo(1.06 / 90, 5)
  })

  it('mark-only bars plot price and keep volume at zero (GitLab #568 AC5)', () => {
    const mark = row({
      trade_count: 0,
      volume_base: '0',
      volume_quote: '0',
      open: '0.9',
      high: '0.9',
      low: '0.9',
      close: '0.9',
    })
    const pts = indexerCandlesToChartPoints([mark])
    expect(pts).toHaveLength(1)
    expect(pts[0].close).toBe(0.9)
    const vol = indexerCandlesToVolumeHistogramPoints([mark], '#0f0', '#f00')
    expect(vol).toHaveLength(1)
    expect(vol[0].value).toBe(0)
  })

  it('omits volume for rows that fail OHLC validation', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [row({ open: '', close: '' }), row({ open: '1', close: '1.1' })],
      '#00ff00',
      '#ff0000'
    )
    expect(pts).toHaveLength(1)
  })

  it('scales 18-dec quote volume to human (GitLab #564)', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [row({ volume_quote: '10000000000000000000', volume_base: '0' })],
      '#00ff00',
      '#ff0000',
      { quoteDecimals: 18, baseDecimals: 6 }
    )
    expect(pts[0]?.value).toBe(10)
    expect(pts[0]?.value).not.toBe(1e19)
  })

  it('falls back to base volume scaled by base decimals when quote is zero', () => {
    const pts = indexerCandlesToVolumeHistogramPoints(
      [row({ volume_quote: '0', volume_base: '1000000' })],
      '#00ff00',
      '#ff0000',
      { quoteDecimals: 18, baseDecimals: 6 }
    )
    expect(pts[0]?.value).toBe(1)
  })

  it('drops bars when scale overflows to non-finite', () => {
    const huge = `1${'0'.repeat(400)}`
    const pts = indexerCandlesToVolumeHistogramPoints(
      [row({ volume_quote: huge, volume_base: '0' })],
      '#00ff00',
      '#ff0000',
      { quoteDecimals: 18, baseDecimals: 6 }
    )
    expect(pts).toHaveLength(0)
  })

  it('does not invert volume as price when scale is applied (C543-8)', () => {
    const a = indexerCandlesToVolumeHistogramPoints(
      [row({ volume_quote: '2000000', volume_base: '0' })],
      '#00ff00',
      '#ff0000',
      { quoteDecimals: 6, baseDecimals: 6 }
    )
    const b = indexerCandlesToVolumeHistogramPoints(
      [row({ volume_quote: '4000000', volume_base: '0' })],
      '#00ff00',
      '#ff0000',
      { quoteDecimals: 6, baseDecimals: 6 }
    )
    expect(a[0]?.value).toBe(2)
    expect(b[0]?.value).toBe(4)
    expect((b[0]?.value ?? 0) / (a[0]?.value ?? 1)).toBe(2)
  })
})
