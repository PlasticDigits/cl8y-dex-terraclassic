import { describe, expect, it } from 'vitest'
import { isTrustedTokenLogoUrl, resolveTrustedTokenLogoUrl } from './tokenLogoAllowlist'

describe('tokenLogoAllowlist (GitLab #378)', () => {
  it('allows PlasticDigits GitLab raw tokenlist images', () => {
    const url = 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png'
    expect(isTrustedTokenLogoUrl(url)).toBe(true)
    expect(resolveTrustedTokenLogoUrl(url)).toBe(url)
  })

  it('rejects untrusted hosts (phishing logos)', () => {
    const evil = 'https://evil.example/logo.png'
    expect(isTrustedTokenLogoUrl(evil)).toBe(false)
    expect(resolveTrustedTokenLogoUrl(evil)).toBeUndefined()
  })

  it('rejects non-https logos', () => {
    expect(isTrustedTokenLogoUrl('http://gitlab.com/logo.png')).toBe(false)
  })
})
