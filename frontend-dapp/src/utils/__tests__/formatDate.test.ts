import { describe, it, expect } from 'vitest'
import { formatTime, formatDateTime } from '../formatDate'

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
