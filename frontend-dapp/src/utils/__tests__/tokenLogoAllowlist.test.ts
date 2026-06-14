import { describe, expect, it } from 'vitest'
import { isAllowedTokenLogoHost, resolveAllowedTokenLogoUri } from '@/utils/tokenLogoAllowlist'

describe('tokenLogoAllowlist', () => {
  it('allows known logo CDN hosts', () => {
    expect(isAllowedTokenLogoHost('gitlab.com')).toBe(true)
    expect(isAllowedTokenLogoHost('raw.githubusercontent.com')).toBe(true)
    expect(isAllowedTokenLogoHost('assets.coingecko.com')).toBe(true)
  })

  it('rejects arbitrary hosts', () => {
    expect(isAllowedTokenLogoHost('evil.example')).toBe(false)
    expect(resolveAllowedTokenLogoUri('https://evil.example/logo.png')).toBeUndefined()
  })

  it('rejects non-https logo URIs', () => {
    expect(resolveAllowedTokenLogoUri('http://assets.coingecko.com/coins/images/1/large/bitcoin.png')).toBeUndefined()
  })

  it('returns allowlisted https URIs unchanged', () => {
    const uri = 'https://assets.coingecko.com/coins/images/1/large/bitcoin.png'
    expect(resolveAllowedTokenLogoUri(uri)).toBe(uri)
  })
})
