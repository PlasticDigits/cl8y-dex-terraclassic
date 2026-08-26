import { describe, it, expect } from 'vitest'
import {
  CHARTS_PAIR_SORT_VOLUME_LABEL,
  composeStatAriaLabel,
  PROTOCOL_VOLUME_DAILY_LABEL,
  PROTOCOL_VOLUME_DAILY_TITLE,
  PROTOCOL_VOLUME_GRAIN_DAILY_LABEL,
  PROTOCOL_VOLUME_GRAIN_HOURLY_LABEL,
  PROTOCOL_VOLUME_GRAIN_MONTHLY_LABEL,
  POOL_VOL_HEADER_TITLE,
  TRAILING_24H_TRADES_TITLE,
  TRAILING_24H_VOLUME_TITLE,
  TRAILING_7D_VOLUME_TITLE,
  TRAILING_30D_VOLUME_TITLE,
  TRAILING_WINDOW_TITLES,
  TRAILING_WINDOW_VISIBLE_LABELS,
  trailingWindowLabelWordCount,
} from '../trailingWindowCopy'

describe('trailingWindowCopy (GitLab #576)', () => {
  it('W3: visible labels stay ≤ 5 words', () => {
    for (const label of TRAILING_WINDOW_VISIBLE_LABELS) {
      expect(trailingWindowLabelWordCount(label), label).toBeLessThanOrEqual(5)
    }
  })

  it('A1/A2/A5: titles are static, have no URLs/env, and do not claim settlement', () => {
    for (const title of TRAILING_WINDOW_TITLES) {
      expect(title).not.toMatch(/https?:|VITE_|localhost|127\.0\.0\.1|indexer\./i)
      expect(title).not.toMatch(/guaranteed|settlement|insured|daily close/i)
      expect(title).toMatch(/last (24 hours|7 days|30 days)/i)
    }
    expect(TRAILING_24H_VOLUME_TITLE).toMatch(/not a midnight reset/i)
    expect(TRAILING_24H_TRADES_TITLE).toMatch(/not a midnight reset/i)
    expect(TRAILING_7D_VOLUME_TITLE).toMatch(/not a calendar-week reset/i)
    expect(TRAILING_30D_VOLUME_TITLE).toMatch(/not a calendar-month reset/i)
    expect(POOL_VOL_HEADER_TITLE).toMatch(/not a midnight reset/i)
  })

  it('sort option stays a trailing 24h label without changing the sort key', () => {
    expect(CHARTS_PAIR_SORT_VOLUME_LABEL).toMatch(/last 24h volume/i)
  })

  it('UTC volume chart copy is calendar grain, not a trailing-window lecture (GitLab #652 / #668)', () => {
    expect(trailingWindowLabelWordCount(PROTOCOL_VOLUME_DAILY_LABEL)).toBeLessThanOrEqual(5)
    expect(trailingWindowLabelWordCount(PROTOCOL_VOLUME_GRAIN_HOURLY_LABEL)).toBeLessThanOrEqual(5)
    expect(trailingWindowLabelWordCount(PROTOCOL_VOLUME_GRAIN_DAILY_LABEL)).toBeLessThanOrEqual(5)
    expect(trailingWindowLabelWordCount(PROTOCOL_VOLUME_GRAIN_MONTHLY_LABEL)).toBeLessThanOrEqual(5)
    expect(PROTOCOL_VOLUME_DAILY_TITLE).toMatch(/UTC calendar/i)
    expect(PROTOCOL_VOLUME_DAILY_TITLE).not.toMatch(/https?:|VITE_|guaranteed|settlement/i)
    expect(PROTOCOL_VOLUME_DAILY_TITLE).not.toMatch(/Last 24h|Last 7d|Last 30d/)
  })

  it('composeStatAriaLabel includes the window and the displayed value', () => {
    expect(composeStatAriaLabel(TRAILING_24H_VOLUME_TITLE, '$1.235K')).toBe(`${TRAILING_24H_VOLUME_TITLE} $1.235K`)
  })
})
