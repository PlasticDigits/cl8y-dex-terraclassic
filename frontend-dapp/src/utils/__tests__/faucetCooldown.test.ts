import { describe, it, expect } from 'vitest'
import { formatFaucetCooldown } from '../faucetCooldown'

describe('formatFaucetCooldown', () => {
  it('formats zero as 0:00', () => {
    expect(formatFaucetCooldown(0)).toBe('0:00')
    expect(formatFaucetCooldown(-5)).toBe('0:00')
  })

  it('formats sub-minute as 0:ss', () => {
    expect(formatFaucetCooldown(5)).toBe('0:05')
    expect(formatFaucetCooldown(59)).toBe('0:59')
  })

  it('formats minutes and seconds as mm:ss', () => {
    expect(formatFaucetCooldown(60)).toBe('1:00')
    expect(formatFaucetCooldown(125)).toBe('2:05')
    expect(formatFaucetCooldown(3599)).toBe('59:59')
  })

  it('formats one hour or more as human string', () => {
    expect(formatFaucetCooldown(3600)).toBe('1h')
    expect(formatFaucetCooldown(3660)).toBe('1h 1m')
    expect(formatFaucetCooldown(7200)).toBe('2h')
    expect(formatFaucetCooldown(5400)).toBe('1h 30m')
  })
})
