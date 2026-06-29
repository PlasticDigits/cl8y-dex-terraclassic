import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { isLcdConnectivityError, probeLcdReachability, LCD_CONNECTIVITY_OUTAGE_MESSAGE } from '../lcdConnectivity'

describe('lcdConnectivity', () => {
  describe('isLcdConnectivityError', () => {
    it('classifies LCD timeout and fetch failures', () => {
      expect(isLcdConnectivityError(new Error('LCD request timed out after 10000ms'))).toBe(true)
      expect(isLcdConnectivityError(new Error('Failed to fetch'))).toBe(true)
      expect(isLcdConnectivityError(new Error('Query failed: 503'))).toBe(true)
    })

    it('does not classify unrelated business errors', () => {
      expect(isLcdConnectivityError(new Error('Insufficient funds'))).toBe(false)
    })
  })

  describe('probeLcdReachability', () => {
    beforeEach(() => {
      vi.stubGlobal('fetch', vi.fn())
    })

    afterEach(() => {
      vi.unstubAllGlobals()
    })

    it('returns true when LCD responds ok', async () => {
      vi.mocked(fetch).mockResolvedValue({ ok: true } as Response)
      await expect(probeLcdReachability()).resolves.toBe(true)
    })

    it('returns false on network failure', async () => {
      vi.mocked(fetch).mockRejectedValue(new Error('Failed to fetch'))
      await expect(probeLcdReachability()).resolves.toBe(false)
    })
  })

  it('exports expected outage copy', () => {
    expect(LCD_CONNECTIVITY_OUTAGE_MESSAGE).toMatch(/connect to the network/i)
  })

  it('reassures users on-chain funds are safe during LCD outage (GitLab #427, SEC-E05)', () => {
    expect(LCD_CONNECTIVITY_OUTAGE_MESSAGE).toMatch(/on-chain/i)
    expect(LCD_CONNECTIVITY_OUTAGE_MESSAGE).toMatch(/unaffected/i)
    expect(LCD_CONNECTIVITY_OUTAGE_MESSAGE).toMatch(/wallet|lp|position/i)
    expect(LCD_CONNECTIVITY_OUTAGE_MESSAGE).toMatch(/only what the app can show/i)
    expect(LCD_CONNECTIVITY_OUTAGE_MESSAGE.toLowerCase()).not.toMatch(/funds at risk|not safe|may lose/i)
  })
})
