import { describe, expect, it } from 'vitest'
import type { IndexerOverview } from '@/types'
import {
  overviewLiquidityDisplay,
  protocolOverviewPollInterval,
  validLiquidityUsd,
  parseLiquiditySnapshot,
  retainQualifiedLiquidity,
  liquiditySnapshotKey,
} from '../protocolLiquidityQuality'
import { pointValueUsd, maxPricedUsd, historicalLiquidityEstimateUsd, maxPlotUsd } from '../protocolVolumeGrain'

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
  it('plots only explicitly reported positive legacy estimates without qualifying them as canonical history', () => {
    const legacy = {
      liquidity_usd: null,
      reported_liquidity_usd: '249000',
      quality: 'unknown' as const,
      reasons: { legacy_unqualified: 1 },
    }
    expect(historicalLiquidityEstimateUsd(legacy)).toBe('249000')
    expect(pointValueUsd(legacy, 'liquidity')).toBeNull()
    expect(maxPricedUsd([legacy], 'liquidity')).toBe(0)
    expect(maxPlotUsd([legacy], 'liquidity')).toBe(249000)
    for (const raw of ['0', '-1', 'NaN', 'Infinity', '1e9', '<script>', '9'.repeat(100)]) {
      expect(historicalLiquidityEstimateUsd({ ...legacy, reported_liquidity_usd: raw })).toBeNull()
    }
    expect(historicalLiquidityEstimateUsd({ ...legacy, reasons: { historical_coverage_unreconciled: 1 } })).toBeNull()
    expect(
      historicalLiquidityEstimateUsd({ ...legacy, reasons: { legacy_unqualified: 1, stale_reserve: 1 } })
    ).toBeNull()
    expect(historicalLiquidityEstimateUsd({ ...legacy, quality: 'fresh' })).toBeNull()
    expect(historicalLiquidityEstimateUsd({ ...legacy, liquidity_usd: '249000' })).toBeNull()
    expect(maxPlotUsd([{ volume_usd: 'Infinity' }, { volume_usd: '-1' }, { volume_usd: '12' }], 'volume')).toBe(12)
  })
  it('polls before normal expiry without an expired-data request storm', () => {
    expect(protocolOverviewPollInterval({ ...fresh, liquidity_as_of: new Date(now).toISOString() }, now)).toBe(10_000)
    expect(
      protocolOverviewPollInterval(
        { ...fresh, liquidity_as_of: new Date(now).toISOString(), liquidity_max_age_seconds: 1 },
        now
      )
    ).toBe(5_000)
    expect(protocolOverviewPollInterval({ ...fresh, liquidity_quality: 'stale' }, now)).toBe(10_000)
    expect(protocolOverviewPollInterval({ ...fresh, liquidity_max_age_seconds: 300 }, now)).toBe(30_000)
    expect(protocolOverviewPollInterval({ ...fresh, liquidity_max_age_seconds: NaN }, now)).toBe(10_000)
    expect(protocolOverviewPollInterval({ ...fresh, liquidity_quality: undefined }, now)).toBe(60_000)
    expect(protocolOverviewPollInterval(fresh, now)).toBe(5_000)
    expect(protocolOverviewPollInterval(fresh, now + 11_000)).toBe(10_000)
  })
  it('permits fresh totals despite permanently unsupported catalog pairs', () => {
    expect(overviewLiquidityDisplay(fresh, now)).toMatchObject({ value: '$249K', comparable: true })
  })
  it('labels partial totals as lower bounds and suppresses deltas', () => {
    const display = overviewLiquidityDisplay(
      { ...fresh, liquidity_quality: 'partial', total_liquidity_usd: '218' },
      now
    )
    expect(display.value).toMatch(/^≥ \$/)
    expect(display.hint).toContain('This is a lower bound.')
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
    expect(display.hint).toContain('The latest pool data is too old.')
    expect(display.hint).toContain('Updated 10 seconds ago.')
    expect(display.hint).not.toContain('2026-')
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
    expect(old.hint).toContain('There is not enough data')
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

describe('public last verified liquidity retention', () => {
  it('retains a complete observation through expiry, missing and degraded totals without a delta', () => {
    const retained = retainQualifiedLiquidity(null, fresh, now)
    expect(retained).toEqual({ value: '249000', quality: 'fresh', asOf: fresh.liquidity_as_of })
    for (const quality of ['partial', 'stale', 'unavailable', 'unknown'] as const) {
      const next = { ...fresh, liquidity_quality: quality, total_liquidity_usd: quality === 'partial' ? '218' : null }
      expect(retainQualifiedLiquidity(retained, next, now)).toBe(retained)
      expect(overviewLiquidityDisplay(next, now, retained)).toMatchObject({ value: '$249K', comparable: false })
    }
    expect(overviewLiquidityDisplay(fresh, now + 60_000, retained)).toMatchObject({ value: '$249K', comparable: false })
    expect(overviewLiquidityDisplay(undefined, now + 60_000, retained).hint).toContain('Last verified total. Updated')
  })
  it('replaces previous totals with qualified decreases and verified zero even when oldest input time is earlier', () => {
    const retained = retainQualifiedLiquidity(null, fresh, now)
    const newer = { ...fresh, liquidity_as_of: new Date(now).toISOString(), total_liquidity_usd: '120000' }
    const reduced = retainQualifiedLiquidity(retained, newer, now)
    expect(reduced?.value).toBe('120000')
    expect(retainQualifiedLiquidity(reduced, { ...fresh, total_liquidity_usd: '110000' }, now)?.value).toBe('110000')
    expect(overviewLiquidityDisplay(fresh, now, reduced)).toMatchObject({ value: '$120K', comparable: false })
    const empty = retainQualifiedLiquidity(
      reduced,
      { ...newer, liquidity_quality: 'verified_empty', total_liquidity_usd: '0' },
      now
    )
    expect(empty?.value).toBe('0')
    expect(overviewLiquidityDisplay(undefined, now, empty)).toMatchObject({ value: '$0', comparable: false })
  })
  it('validates cached provenance, timestamp, quality and value; namespaces distinct deployments', () => {
    const retained = retainQualifiedLiquidity(null, fresh, now)!
    expect(parseLiquiditySnapshot(JSON.stringify(retained), now)).toEqual(retained)
    for (const bad of [
      null,
      '',
      '{}',
      '[1]',
      'null',
      '<script>',
      JSON.stringify({ ...retained, quality: 'partial' }),
      JSON.stringify({ ...retained, quality: 'verified_empty' }),
      JSON.stringify({ ...retained, value: 'Infinity' }),
      JSON.stringify({ ...retained, asOf: 'bogus' }),
      JSON.stringify({ ...retained, asOf: '1' }),
      JSON.stringify({ ...retained, asOf: '2026-02-30T00:00:00Z' }),
      JSON.stringify({ ...retained, asOf: new Date(now + 1).toISOString() }),
      JSON.stringify({ ...retained, wallet: 'private' }),
    ]) {
      expect(parseLiquiditySnapshot(bad, now)).toBeNull()
    }
    expect(liquiditySnapshotKey('columbus-5', 'factory-a', 'indexer-a')).not.toBe(
      liquiditySnapshotKey('localterra', 'factory-a', 'indexer-a')
    )
    expect(liquiditySnapshotKey('columbus-5', 'factory-a', 'indexer-a')).not.toBe(
      liquiditySnapshotKey('columbus-5', 'factory-b', 'indexer-a')
    )
    expect(liquiditySnapshotKey('columbus-5', 'factory-a', 'indexer-a')).not.toBe(
      liquiditySnapshotKey('columbus-5', 'factory-a', 'indexer-b')
    )
  })
})
