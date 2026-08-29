import { describe, it, expect } from 'vitest'
import {
  formatChartsOverviewCount,
  formatChartsOverviewUstcUsd,
  formatChartsOverviewVolumeUsd,
  formatIndexedVolumeUsd,
  formatPairListVolumeUsd,
} from '../chartsOverviewStats'

describe('formatChartsOverviewVolumeUsd (GitLab #548)', () => {
  it('F1: compact USD with $ prefix; does not rescale human dollars', () => {
    expect(formatChartsOverviewVolumeUsd('1234.56', 4)).toBe('$1.235K')
  })

  it('F2: zero USD with trades is unpriced em dash', () => {
    expect(formatChartsOverviewVolumeUsd('0', 4)).toBe('—')
    expect(formatChartsOverviewVolumeUsd('0.00', 1)).toBe('—')
  })

  it('F3: idle DEX shows $0', () => {
    expect(formatChartsOverviewVolumeUsd('0', 0)).toBe('$0')
  })

  it('F4: missing/null/empty is em dash', () => {
    expect(formatChartsOverviewVolumeUsd(null, 4)).toBe('—')
    expect(formatChartsOverviewVolumeUsd(undefined, 4)).toBe('—')
    expect(formatChartsOverviewVolumeUsd('', 4)).toBe('—')
  })

  it('F9/A4: HTML, NaN, overflow, huge strings → em dash', () => {
    expect(formatChartsOverviewVolumeUsd('"><script>', 4)).toBe('—')
    expect(formatChartsOverviewVolumeUsd('NaN', 4)).toBe('—')
    expect(formatChartsOverviewVolumeUsd('1e309', 4)).toBe('—')
    expect(formatChartsOverviewVolumeUsd('not-a-number', 4)).toBe('—')
    expect(formatChartsOverviewVolumeUsd('9'.repeat(80), 4)).toBe('—')
  })

  it('A12: negative USD is hidden', () => {
    expect(formatChartsOverviewVolumeUsd('-1', 4)).toBe('—')
  })

  it('A6: already-human USD is not ×1e6', () => {
    expect(formatChartsOverviewVolumeUsd('0.12', 4)).toBe('$0.1200')
  })
})

describe('formatChartsOverviewUstcUsd (GitLab #548 C4)', () => {
  it('F5: $ prefix, no T compact, null/empty → em dash', () => {
    const shown = formatChartsOverviewUstcUsd('0.004878')
    expect(shown.startsWith('$')).toBe(true)
    expect(shown).not.toMatch(/\dT\b/)
    expect(shown).toContain('0.004878')
    expect(formatChartsOverviewUstcUsd(null)).toBe('—')
    expect(formatChartsOverviewUstcUsd('')).toBe('—')
  })

  it('never uses LUNC magnitude as USTC', () => {
    expect(formatChartsOverviewUstcUsd('0.004878')).not.toBe('$0.00005')
  })
})

describe('formatChartsOverviewCount', () => {
  it('F6: locale integers, no compact T', () => {
    expect(formatChartsOverviewCount(4)).toBe('4')
    expect(formatChartsOverviewCount(13)).toBe('13')
    expect(formatChartsOverviewCount(12)).toBe('12')
    expect(formatChartsOverviewCount(1000)).toBe('1,000')
    expect(formatChartsOverviewCount(1e12)).not.toMatch(/T/)
  })
})

describe('formatIndexedVolumeUsd (GitLab #553)', () => {
  it('matches Charts overview contract and does not compact raw 18-dec as T', () => {
    expect(formatIndexedVolumeUsd('711.2', 4)).toBe(formatChartsOverviewVolumeUsd('711.2', 4))
    expect(formatIndexedVolumeUsd('711.2', 4)).toMatch(/^\$/)
    expect(formatIndexedVolumeUsd('711.2', 4)).not.toMatch(/T/)
    expect(formatIndexedVolumeUsd(null, 4)).toBe('—')
    expect(formatIndexedVolumeUsd('0', 4)).toBe('—')
    expect(formatIndexedVolumeUsd('0', 0)).toBe('$0')
  })
})

describe('formatPairListVolumeUsd (GitLab #692)', () => {
  it('shows compact USD or em-dash; idle and hostile never $0 or HTML', () => {
    expect(formatPairListVolumeUsd('12400')).toMatch(/^\$/)
    expect(formatPairListVolumeUsd('0')).toBe('—')
    expect(formatPairListVolumeUsd(undefined)).toBe('—')
    expect(formatPairListVolumeUsd('Infinity')).toBe('—')
    expect(formatPairListVolumeUsd('-1')).toBe('—')
    expect(formatPairListVolumeUsd('<script>alert(1)</script>')).toBe('—')
  })
})
