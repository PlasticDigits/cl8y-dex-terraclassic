import { describe, expect, it } from 'vitest'
import {
  cssContainsForbiddenOrangeWash,
  headingWashFromTokenId,
  hueFromTokenId,
  washRgbaCss,
  clampWashRgb,
  FORBIDDEN_ORANGE_WASH_RGB,
  NEUTRAL_TICKET_HEADER_BACKGROUND,
  sampleLogoWashRgba,
} from '../tokenHeadingWash'

describe('tokenHeadingWash (GitLab #693)', () => {
  it('hash wash is stable per id, never the leftover orange, and numeric rgba only', () => {
    const a = headingWashFromTokenId('terra1aaa0000000000000000000000000000001')
    const b = headingWashFromTokenId('terra1aaa0000000000000000000000000000001')
    const c = headingWashFromTokenId('terra1bbb0000000000000000000000000000002')
    expect(a).toBe(b)
    expect(a).not.toBe(c)
    expect(cssContainsForbiddenOrangeWash(a)).toBe(false)
    expect(a).toMatch(/rgba\(\d+, \d+, \d+, 0\.16\)/)
    expect(a).not.toMatch(/url\(/)
    expect(headingWashFromTokenId('')).toBe(NEUTRAL_TICKET_HEADER_BACKGROUND)
  })

  it('skips the warm-orange hue band', () => {
    for (const id of ['uluna', 'uusd', 'terra1orange', 'AAA', 'UST1']) {
      const hue = hueFromTokenId(id)
      expect(hue < 18 || hue > 48).toBe(true)
    }
  })

  it('clampWashRgb never returns the forbidden orange triple (A4)', () => {
    const [r, g, b] = clampWashRgb(...FORBIDDEN_ORANGE_WASH_RGB)
    expect([r, g, b]).not.toEqual([...FORBIDDEN_ORANGE_WASH_RGB])
    const neon = washRgbaCss(0, 255, 0)
    expect(cssContainsForbiddenOrangeWash(neon)).toBe(false)
    const white = washRgbaCss(255, 255, 255)
    expect(white).toMatch(/rgba\(\d+, \d+, \d+,/)
  })

  it('sampleLogoWashRgba returns null on tainted canvas instead of throwing (A3)', () => {
    const img = document.createElement('img')
    expect(() => sampleLogoWashRgba(img)).not.toThrow()
  })
})
