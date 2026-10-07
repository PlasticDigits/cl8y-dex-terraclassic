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
  it('plots available amounts with canonical values preferred over reported fallback', () => {
    expect(pointValueUsd({ liquidity_usd: null, reported_liquidity_usd: '249000' }, 'liquidity')).toBe('249000')
    expect(pointValueUsd({ liquidity_usd: '218', reported_liquidity_usd: '249000' }, 'liquidity')).toBe('218')
    expect(pointValueUsd({ liquidity_usd: '0', reported_liquidity_usd: '249000' }, 'liquidity')).toBe('0')
    for (const raw of ['-1', 'NaN', 'Infinity', '1e9', '<script>', '9'.repeat(100), '']) {
      expect(pointValueUsd({ liquidity_usd: raw, reported_liquidity_usd: raw }, 'liquidity')).toBeNull()
    }
    expect(maxPricedUsd([{ volume_usd: 'Infinity' }, { volume_usd: '-1' }, { volume_usd: '12' }], 'volume')).toBe(12)
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
    expect(display.hint).toBe('Updated 10 seconds ago.')
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
    expect(display.hint).toContain('Updated 10 seconds ago.')
    expect(display.hint).not.toContain('2026-')
    expect(display.comparable).toBe(false)
  })
  it('rejects contradictory verified-empty positive totals in the headline', () => {
    const display = overviewLiquidityDisplay({ ...fresh, liquidity_quality: 'verified_empty' }, now)
    expect(display).toMatchObject({ quality: 'unknown', value: '—', comparable: false })
  })
  it('fails closed on malformed numerics and statuses while safely supporting legacy positives', () => {
    for (const raw of ['NaN', 'Infinity', '-1', '1e5', '<script>x</script>', '9'.repeat(100), '']) {
      expect(validLiquidityUsd(raw)).toBeNull()
      expect(overviewLiquidityDisplay({ ...fresh, total_liquidity_usd: raw }, now).value).toBe('—')
    }
    const old = overviewLiquidityDisplay({ ...fresh, liquidity_quality: undefined }, now)
    expect(old.hint).toBe('Updated 10 seconds ago.')
    expect(old.comparable).toBe(false)
  })
  it('plots all supplied numeric history without imposing current quality on older dates', () => {
    for (const quality of ['partial', 'stale', 'unavailable', 'unknown', 'fresh', 'verified_empty'] as const) {
      expect(pointValueUsd({ liquidity_usd: '218', quality }, 'liquidity')).toBe('218')
    }
    expect(pointValueUsd({ liquidity_usd: null }, 'liquidity')).toBeNull()
    expect(pointValueUsd({ liquidity_usd: '0' }, 'liquidity')).toBe('0')
    expect(pointValueUsd({ liquidity_usd: '218.57', priced_pair_count: 2 }, 'liquidity')).toBe('218.57')
    expect(
      maxPricedUsd(
        [
          { liquidity_usd: '250', quality: 'fresh' },
          { liquidity_usd: '9999', quality: 'partial' },
        ],
        'liquidity'
      )
    ).toBe(9999)
  })
})

describe('public last verified liquidity retention', () => {
  it('keeps an initial qualified partial lower bound through TTL, null and outages until a complete total arrives', () => {
    const partial = { ...fresh, liquidity_quality: 'partial' as const, total_liquidity_usd: '206000' }
    const retained = retainQualifiedLiquidity(null, partial, now)
    expect(retained).toEqual({ value: '206000', quality: 'partial', asOf: partial.liquidity_as_of })
    expect(parseLiquiditySnapshot(JSON.stringify(retained), now + 60_000)).toEqual(retained)
    expect(overviewLiquidityDisplay(partial, now, retained)).toMatchObject({
      value: '≥ $206K',
      comparable: false,
    })
    for (const next of [
      partial,
      undefined,
      { ...partial, total_liquidity_usd: null },
      { ...partial, liquidity_quality: 'unknown' as const },
    ]) {
      expect(retainQualifiedLiquidity(retained, next, now + 60_000)).toBe(retained)
      expect(overviewLiquidityDisplay(next, now + 60_000, retained)).toMatchObject({
        quality: 'partial',
        value: '≥ $206K',
        comparable: false,
      })
      expect(overviewLiquidityDisplay(next, now + 60_000, retained).hint).toMatch(/^Updated /)
    }
    const lower = { ...partial, total_liquidity_usd: '120000', liquidity_as_of: new Date(now).toISOString() }
    const reduced = retainQualifiedLiquidity(retained, lower, now)
    expect(reduced?.value).toBe('120000')
    expect(overviewLiquidityDisplay(lower, now, reduced).value).toBe('≥ $120K')
    // A full total takes priority even when its value or oldest input time is lower.
    expect(overviewLiquidityDisplay(fresh, now, reduced)).toMatchObject({ value: '$249K', comparable: true })
    const complete = retainQualifiedLiquidity(reduced, fresh, now)
    expect(complete?.quality).toBe('fresh')
    expect(retainQualifiedLiquidity(complete, lower, now)).toBe(complete)
    expect(overviewLiquidityDisplay(lower, now, complete).value).toBe('$249K')
    const empty = { ...fresh, total_liquidity_usd: '0', liquidity_quality: 'verified_empty' as const }
    expect(overviewLiquidityDisplay(empty, now, reduced)).toMatchObject({ value: '$0', comparable: true })
    expect(retainQualifiedLiquidity(reduced, empty, now)?.quality).toBe('verified_empty')
  })
  it('retains a complete observation through expiry, missing and degraded totals without a delta', () => {
    const retained = retainQualifiedLiquidity(null, fresh, now)
    expect(retained).toEqual({ value: '249000', quality: 'fresh', asOf: fresh.liquidity_as_of })
    for (const quality of ['partial', 'stale', 'unavailable', 'unknown'] as const) {
      const next = { ...fresh, liquidity_quality: quality, total_liquidity_usd: quality === 'partial' ? '218' : null }
      expect(retainQualifiedLiquidity(retained, next, now)).toBe(retained)
      expect(overviewLiquidityDisplay(next, now, retained)).toMatchObject({ value: '$249K', comparable: false })
    }
    expect(overviewLiquidityDisplay(fresh, now + 60_000, retained)).toMatchObject({ value: '$249K', comparable: false })
    expect(overviewLiquidityDisplay(undefined, now + 60_000, retained).hint).toBe('Updated 1 minute ago.')
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
      JSON.stringify({ ...retained, quality: 'unknown' }),
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
