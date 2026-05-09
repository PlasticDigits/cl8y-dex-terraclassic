import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  hasRiskAcknowledgement,
  RISK_ACK_STORAGE_KEY,
  RISK_ACK_VERSION,
  readRiskAcknowledgement,
  setRiskAcknowledged,
} from '@/utils/riskAcknowledgement'

describe('riskAcknowledgement', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  afterEach(() => {
    localStorage.clear()
    vi.unstubAllEnvs()
  })

  it('returns false when storage empty', () => {
    expect(hasRiskAcknowledgement()).toBe(false)
    expect(readRiskAcknowledgement()).toBeNull()
  })

  it('returns false when version is below current', () => {
    localStorage.setItem(RISK_ACK_STORAGE_KEY, JSON.stringify({ v: RISK_ACK_VERSION - 1 }))
    expect(hasRiskAcknowledgement()).toBe(false)
  })

  it('returns true after setRiskAcknowledged', () => {
    setRiskAcknowledged()
    expect(hasRiskAcknowledgement()).toBe(true)
    expect(readRiskAcknowledgement()?.v).toBe(RISK_ACK_VERSION)
  })

  it('ignores malformed JSON', () => {
    localStorage.setItem(RISK_ACK_STORAGE_KEY, 'not-json')
    expect(readRiskAcknowledgement()).toBeNull()
    expect(hasRiskAcknowledgement()).toBe(false)
  })
})
