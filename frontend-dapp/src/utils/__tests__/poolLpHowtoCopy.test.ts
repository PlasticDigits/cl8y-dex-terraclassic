import { describe, expect, it } from 'vitest'
import {
  forbiddenPoolLpHowtoCopyHits,
  poolLpHowtoAllText,
  POOL_LP_HOWTO_CREATE_PAIR_FEE,
  POOL_LP_HOWTO_GAS,
  POOL_LP_HOWTO_HREF,
  POOL_LP_HOWTO_LIMITS,
  POOL_LP_HOWTO_LINKS,
  POOL_LP_HOWTO_NO_INCENTIVE,
  POOL_LP_HOWTO_RATIO_DONATE,
  POOL_LP_HOWTO_TWO_SIDED,
  POOL_LP_HOWTO_UNWRAP,
  POOL_LP_HOWTO_WITHDRAW,
  POOL_LP_HOWTO_WRAP,
} from '../poolLpHowtoCopy'

describe('poolLpHowtoCopy (#531)', () => {
  const text = poolLpHowtoAllText()

  it('has no forbidden farm / protocol / phishing copy (A2 / A10 / #489)', () => {
    expect(forbiddenPoolLpHowtoCopyHits()).toEqual([])
    expect(text).not.toMatch(/https?:\/\//i)
    expect(text).not.toMatch(/qr\s*code/i)
  })

  it('states two-sided provide, wrap or auto-wrap, gas, withdraw, and no incentive', () => {
    expect(POOL_LP_HOWTO_TWO_SIDED).toMatch(/both tokens/i)
    expect(POOL_LP_HOWTO_TWO_SIDED).toMatch(/LUNC-only/i)
    expect(POOL_LP_HOWTO_WRAP).toMatch(/Use native LUNC \(auto-wrap\)/)
    expect(POOL_LP_HOWTO_WRAP).toMatch(/Wrap/)
    expect(POOL_LP_HOWTO_GAS).toMatch(/gas/i)
    expect(POOL_LP_HOWTO_WITHDRAW).toMatch(/Withdraw/)
    expect(POOL_LP_HOWTO_WITHDRAW).toMatch(/LP tokens/i)
    expect(POOL_LP_HOWTO_NO_INCENTIVE).toMatch(/no LP or maker incentive program/i)
  })

  it('disambiguates limits as maker escrow, not LP', () => {
    expect(POOL_LP_HOWTO_LIMITS).toMatch(/limit/i)
    expect(POOL_LP_HOWTO_LIMITS).toMatch(/not a pool share/i)
    expect(POOL_LP_HOWTO_LIMITS).not.toMatch(/\bLP shares\b/)
  })

  it('mentions pair-creation fee and off-ratio donate', () => {
    expect(POOL_LP_HOWTO_CREATE_PAIR_FEE).toMatch(/creation fee/i)
    expect(POOL_LP_HOWTO_RATIO_DONATE).toMatch(/donated/i)
    expect(POOL_LP_HOWTO_UNWRAP).toMatch(/not free/i)
  })

  it('uses in-app links only', () => {
    expect(POOL_LP_HOWTO_HREF).toBe('/pool#lp-howto')
    expect(POOL_LP_HOWTO_LINKS.map((l) => l.href)).toEqual(['/wrap', '/trade', '/limits', '/create'])
    for (const link of POOL_LP_HOWTO_LINKS) {
      expect(link.href.startsWith('/')).toBe(true)
      expect(link.href).not.toMatch(/^https?:/i)
    }
  })
})
