import { describe, it, expect } from 'vitest'
import { formatCreatedAtTitle, formatDateTime, formatRelativeAge, formatTime, parseCreatedAtMs } from '../formatDate'

const NOW = Date.parse('2026-08-26T12:00:00.000Z')

describe('formatTime', () => {
  it('formats an ISO time string', () => {
    const result = formatTime('2025-01-15T14:30:45Z')
    expect(result).toMatch(/\d{1,2}:\d{2}:\d{2}/)
  })
})

describe('formatDateTime', () => {
  it('returns dash for null', () => {
    expect(formatDateTime(null)).toBe('—')
  })

  it('formats a valid ISO string', () => {
    const result = formatDateTime('2025-01-15T14:30:00Z')
    expect(result).toContain('Jan')
    expect(result).toContain('15')
  })
})

describe('formatRelativeAge (GitLab #662)', () => {
  it('locks buckets for injected now', () => {
    expect(formatRelativeAge('2026-08-26T11:59:30.000Z', NOW)).toBe('just now')
    expect(formatRelativeAge('2026-08-26T11:55:00.000Z', NOW)).toBe('5 minutes ago')
    expect(formatRelativeAge('2026-08-26T09:00:00.000Z', NOW)).toBe('3 hours ago')
    expect(formatRelativeAge('2026-08-24T12:00:00.000Z', NOW)).toBe('2 days ago')
    expect(formatRelativeAge('2025-07-22T12:00:00.000Z', NOW)).toBe('1 year ago') // 400 days
    expect(formatRelativeAge('2024-06-17T12:00:00.000Z', NOW)).toBe('2 years ago') // 800 days
  })

  it('singular vs plural', () => {
    expect(formatRelativeAge('2026-08-26T11:59:00.000Z', NOW)).toBe('1 minute ago')
    expect(formatRelativeAge('2026-08-26T11:00:00.000Z', NOW)).toBe('1 hour ago')
    expect(formatRelativeAge('2026-08-25T12:00:00.000Z', NOW)).toBe('1 day ago')
  })

  it('clamps near-future skew to just now and rejects far future', () => {
    expect(formatRelativeAge('2026-08-26T12:00:30.000Z', NOW)).toBe('just now')
    expect(formatRelativeAge('2026-08-26T12:01:59.000Z', NOW)).toBe('just now')
    expect(formatRelativeAge('2036-08-26T12:00:00.000Z', NOW)).toBe('—')
  })

  it('returns em-dash for missing, empty, and garbage payloads', () => {
    expect(formatRelativeAge(null, NOW)).toBe('—')
    expect(formatRelativeAge(undefined, NOW)).toBe('—')
    expect(formatRelativeAge('', NOW)).toBe('—')
    expect(formatRelativeAge('not-a-date', NOW)).toBe('—')
    expect(formatRelativeAge('<img>', NOW)).toBe('—')
    expect(formatRelativeAge('<script>alert(1)</script>', NOW)).toBe('—')
    expect(formatRelativeAge('"><img src=x onerror=alert(1)>', NOW)).toBe('—')
    expect(formatRelativeAge('javascript:alert(1)', NOW)).toBe('—')
    expect(formatRelativeAge('NaN', NOW)).toBe('—')
    expect(formatRelativeAge('Infinity', NOW)).toBe('—')
    expect(formatRelativeAge('0000-01-01T00:00:00Z', NOW)).toBe('—')
    expect(formatRelativeAge('+275760-09-13T00:00:00Z', NOW)).toBe('—')
    expect(formatRelativeAge('{"nested":true}', NOW)).toBe('—')
    expect(formatRelativeAge('a'.repeat(200), NOW)).toBe('—')
  })

  it('rejects unix-ms / unix-s numeric strings so ages cannot be millennia', () => {
    expect(formatRelativeAge('1710000000000', NOW)).toBe('—')
    expect(formatRelativeAge('1710000000', NOW)).toBe('—')
    expect(formatRelativeAge('1710000000000000000', NOW)).toBe('—')
  })

  it('parses Z and +00:00 offsets', () => {
    expect(formatRelativeAge('2026-08-26T09:00:00Z', NOW)).toBe('3 hours ago')
    expect(formatRelativeAge('2026-08-26T09:00:00+00:00', NOW)).toBe('3 hours ago')
  })

  it('does not treat a number as ISO', () => {
    expect(formatRelativeAge(1710000000000 as unknown as string, NOW)).toBe('—')
  })
})

describe('parseCreatedAtMs / formatCreatedAtTitle (GitLab #662)', () => {
  it('parses valid ISO and omits title on garbage', () => {
    expect(parseCreatedAtMs('2026-08-26T09:00:00.000Z')).toBe(Date.parse('2026-08-26T09:00:00.000Z'))
    expect(formatCreatedAtTitle('2026-08-26T09:00:00.000Z')).toEqual(expect.any(String))
    expect(formatCreatedAtTitle('<script>alert(1)</script>')).toBeUndefined()
    expect(formatCreatedAtTitle('javascript:alert(1)')).toBeUndefined()
    expect(formatCreatedAtTitle('"><img src=x>')).toBeUndefined()
  })

  it('title is locale text, never the raw payload', () => {
    const iso = '2026-08-26T09:00:00.000Z'
    const title = formatCreatedAtTitle(iso)
    expect(title).toBeTruthy()
    expect(title).not.toBe(iso)
    expect(title).not.toMatch(/<|javascript:/i)
  })
})
