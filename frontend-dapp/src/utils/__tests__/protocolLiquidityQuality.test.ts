import { describe, expect, it } from 'vitest'
import type { IndexerOverview } from '@/types'
import { overviewLiquidityDisplay, validLiquidityUsd } from '../protocolLiquidityQuality'
import { pointValueUsd, maxPricedUsd } from '../protocolVolumeGrain'

const now = Date.parse('2026-10-04T08:00:00Z')
const fresh: IndexerOverview = {
  total_volume_24h: '0',
  total_trades_24h: 0,
  pair_count: 22,
  token_count: 12,
  total_liquidity_usd: '249000',
  liquidity_quality: 'fresh',
  liquidity_as_of: '2026-10-04T07:59:50Z',
  liquidity_max_age_seconds: 20,
  liquidity_change_24h_pct: '-1',
  priced_pair_count: 11,
  unpriced_pair_count: 11,
}

describe('liquidity quality #1380', () => {
  it('permits fresh totals despite permanently unsupported catalog pairs', () => {
    expect(overviewLiquidityDisplay(fresh, now)).toMatchObject({ value: '$249K', comparable: true })
  })
  it('labels partial totals as lower bounds and suppresses deltas', () => {
    const display = overviewLiquidityDisplay(
      { ...fresh, liquidity_quality: 'partial', total_liquidity_usd: '218' },
      now
    )
    expect(display.value).toMatch(/^≥ \$/)
    expect(display.hint).toContain('Partial · lower bound')
    expect(display.comparable).toBe(false)
  })
  it('ages liquidity independently and rejects future/missing observations', () => {
    expect(overviewLiquidityDisplay(fresh, now + 11000)).toMatchObject({ quality: 'stale', comparable: false })
    expect(overviewLiquidityDisplay({ ...fresh, liquidity_as_of: undefined }, now).quality).toBe('unavailable')
    expect(overviewLiquidityDisplay({ ...fresh, liquidity_as_of: '2026-10-05T08:00:00Z' }, now).value).toBe('—')
  })
  it('distinguishes unknown old zero from explicit verified empty and real withdrawal', () => {
    expect(
      overviewLiquidityDisplay(
        { ...fresh, liquidity_quality: undefined, total_liquidity_usd: '0', priced_pair_count: 0 },
        now
      ).value
    ).toBe('—')
    expect(
      overviewLiquidityDisplay({ ...fresh, liquidity_quality: 'verified_empty', total_liquidity_usd: '0' }, now).value
    ).toBe('$0')
    expect(overviewLiquidityDisplay({ ...fresh, total_liquidity_usd: '0' }, now).value).toBe('$0')
  })
  it('keeps stale last-observed value explicitly dated without a delta', () => {
    const display = overviewLiquidityDisplay({ ...fresh, liquidity_quality: 'stale' }, now)
    expect(display.hint).toContain('Stale · current liquidity unknown')
    expect(display.hint).toContain('2026-10-04T07:59:50.000Z')
    expect(display.comparable).toBe(false)
  })
  it('rejects contradictory verified-empty positive totals in overview and history', () => {
    const display = overviewLiquidityDisplay({ ...fresh, liquidity_quality: 'verified_empty' }, now)
    expect(display).toMatchObject({ quality: 'unknown', value: '—', comparable: false })
    expect(pointValueUsd({ quality: 'verified_empty', liquidity_usd: '250' }, 'liquidity')).toBeNull()
    expect(maxPricedUsd([{ quality: 'verified_empty', liquidity_usd: '250' }], 'liquidity')).toBe(0)
  })
  it('fails closed on malformed numerics and statuses while safely supporting legacy positives', () => {
    for (const raw of ['NaN', 'Infinity', '-1', '1e5', '<script>x</script>', '9'.repeat(100), '']) {
      expect(validLiquidityUsd(raw)).toBeNull()
      expect(overviewLiquidityDisplay({ ...fresh, total_liquidity_usd: raw }, now).value).toBe('—')
    }
    const old = overviewLiquidityDisplay({ ...fresh, liquidity_quality: undefined }, now)
    expect(old.hint).toContain('Coverage unknown')
    expect(old.comparable).toBe(false)
  })
  it('gaps degraded/unknown history without aging good terminal samples to today', () => {
    for (const quality of ['partial', 'stale', 'unavailable', 'unknown'] as const) {
      expect(pointValueUsd({ liquidity_usd: '218', quality }, 'liquidity')).toBeNull()
    }
    expect(pointValueUsd({ liquidity_usd: '0', priced_pair_count: 0 }, 'liquidity')).toBeNull()
    expect(pointValueUsd({ liquidity_usd: '218.57', priced_pair_count: 2 }, 'liquidity')).toBeNull()
    expect(maxPricedUsd([{ liquidity_usd: '218.57', priced_pair_count: 2 }], 'liquidity')).toBe(0)
    expect(pointValueUsd({ liquidity_usd: '0', quality: 'verified_empty' }, 'liquidity')).toBe('0')
    expect(pointValueUsd({ liquidity_usd: '250', quality: 'fresh', as_of: '2020-01-01T00:00:00Z' }, 'liquidity')).toBe(
      '250'
    )
    expect(
      maxPricedUsd(
        [
          { liquidity_usd: '250', quality: 'fresh' },
          { liquidity_usd: '9999', quality: 'partial' },
        ],
        'liquidity'
      )
    ).toBe(250)
  })
})
