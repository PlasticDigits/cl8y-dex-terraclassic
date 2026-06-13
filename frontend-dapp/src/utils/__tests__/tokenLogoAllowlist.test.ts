import { describe, expect, it } from 'vitest'
import { isAllowedLogoUrl, resolveSafeLogoUrl } from '../tokenLogoAllowlist'

describe('tokenLogoAllowlist', () => {
  it('allows trusted logo hosts', () => {
    expect(
      isAllowedLogoUrl('https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png')
    ).toBe(true)
    expect(isAllowedLogoUrl('https://assets.coingecko.com/coins/images/1/large/bitcoin.png')).toBe(true)
  })

  it('rejects untrusted hosts and non-https', () => {
    expect(isAllowedLogoUrl('https://evil.example/logo.png')).toBe(false)
    expect(isAllowedLogoUrl('http://gitlab.com/logo.png')).toBe(false)
    expect(isAllowedLogoUrl('not-a-url')).toBe(false)
  })

  it('resolveSafeLogoUrl returns undefined for blocked URLs', () => {
    expect(resolveSafeLogoUrl('https://evil.example/logo.png')).toBeUndefined()
    expect(
      resolveSafeLogoUrl('https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png')
    ).toContain('gitlab.com')
  })
})
