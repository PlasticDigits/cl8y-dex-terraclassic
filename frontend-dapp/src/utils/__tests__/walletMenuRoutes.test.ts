import { describe, expect, it } from 'vitest'
import { traderProfilePath, WALLET_PORTFOLIO_PATH } from '../walletMenuRoutes'

const SAMPLE = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

describe('walletMenuRoutes (GitLab #671)', () => {
  it('keeps Portfolio on a same-origin app path', () => {
    expect(WALLET_PORTFOLIO_PATH).toBe('/portfolio')
  })

  it('builds /trader/{bech32} for a valid Terra address', () => {
    expect(traderProfilePath(SAMPLE)).toBe(`/trader/${SAMPLE}`)
  })

  it('omits trader href for empty, protocol-relative, and scheme-injected values', () => {
    expect(traderProfilePath('')).toBeNull()
    expect(traderProfilePath('//evil.example')).toBeNull()
    expect(traderProfilePath('https://evil.example')).toBeNull()
    expect(traderProfilePath('javascript:alert(1)')).toBeNull()
    expect(traderProfilePath(`${SAMPLE}/../`)).toBeNull()
  })
})
