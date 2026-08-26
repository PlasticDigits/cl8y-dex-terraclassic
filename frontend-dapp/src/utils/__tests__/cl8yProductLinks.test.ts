import { describe, expect, it } from 'vitest'
import {
  CL8Y_PRODUCT_BRIDGE_HREF,
  CL8Y_PRODUCT_HOME_HREF,
  CL8Y_PRODUCT_LINKS,
  canonicalCl8yProductHref,
  isAllowedCl8yProductHref,
} from '@/utils/cl8yProductLinks'

describe('isAllowedCl8yProductHref (GitLab #663)', () => {
  it('accepts the two pinned HTTPS URLs with or without a trailing slash', () => {
    expect(isAllowedCl8yProductHref(CL8Y_PRODUCT_HOME_HREF)).toBe(true)
    expect(isAllowedCl8yProductHref(CL8Y_PRODUCT_BRIDGE_HREF)).toBe(true)
    expect(isAllowedCl8yProductHref('https://cl8y.com')).toBe(true)
    expect(isAllowedCl8yProductHref('https://bridge.cl8y.com')).toBe(true)
    expect(canonicalCl8yProductHref('https://cl8y.com')).toBe(CL8Y_PRODUCT_HOME_HREF)
    expect(canonicalCl8yProductHref('https://bridge.cl8y.com')).toBe(CL8Y_PRODUCT_BRIDGE_HREF)
  })

  it('rejects protocol smuggling and relative URLs', () => {
    expect(isAllowedCl8yProductHref('http://cl8y.com')).toBe(false)
    expect(isAllowedCl8yProductHref('http://cl8y.com/')).toBe(false)
    expect(isAllowedCl8yProductHref('//bridge.cl8y.com')).toBe(false)
    expect(isAllowedCl8yProductHref('javascript:alert(1)')).toBe(false)
    expect(isAllowedCl8yProductHref('data:text/html,hi')).toBe(false)
    expect(isAllowedCl8yProductHref('vbscript:alert(1)')).toBe(false)
    expect(isAllowedCl8yProductHref('/bridge')).toBe(false)
    expect(isAllowedCl8yProductHref('')).toBe(false)
  })

  it('rejects lookalike hosts, open redirects, and userinfo', () => {
    expect(isAllowedCl8yProductHref('https://cl8y.com.evil')).toBe(false)
    expect(isAllowedCl8yProductHref('https://cl8y.com.evil/')).toBe(false)
    expect(isAllowedCl8yProductHref('https://bridge.cl8y.com.attacker')).toBe(false)
    expect(isAllowedCl8yProductHref('https://evil.com/?u=https://cl8y.com')).toBe(false)
    expect(isAllowedCl8yProductHref('https://user:pass@cl8y.com')).toBe(false)
    expect(isAllowedCl8yProductHref('https://user:pass@cl8y.com/')).toBe(false)
    expect(isAllowedCl8yProductHref('https://cI8y.com/')).toBe(false)
    expect(isAllowedCl8yProductHref('https://xn--l8y-7cd.com/')).toBe(false)
    expect(isAllowedCl8yProductHref('https://www.cl8y.com/')).toBe(false)
    expect(isAllowedCl8yProductHref('https://dex.cl8y.com/')).toBe(false)
  })

  it('rejects extra path, query, hash, and non-default port', () => {
    expect(isAllowedCl8yProductHref('https://cl8y.com/bridge')).toBe(false)
    expect(isAllowedCl8yProductHref('https://cl8y.com/?q=1')).toBe(false)
    expect(isAllowedCl8yProductHref('https://cl8y.com/#x')).toBe(false)
    expect(isAllowedCl8yProductHref('https://bridge.cl8y.com:444/')).toBe(false)
  })
})

describe('CL8Y_PRODUCT_LINKS allowlist (GitLab #663)', () => {
  it('pins exactly Homepage + Bridge with distinct labels', () => {
    expect(CL8Y_PRODUCT_LINKS).toHaveLength(2)
    expect(CL8Y_PRODUCT_LINKS.map((link) => link.id)).toEqual(['home', 'bridge'])
    expect(CL8Y_PRODUCT_LINKS[0]?.href).toBe(CL8Y_PRODUCT_HOME_HREF)
    expect(CL8Y_PRODUCT_LINKS[1]?.href).toBe(CL8Y_PRODUCT_BRIDGE_HREF)
    for (const link of CL8Y_PRODUCT_LINKS) {
      expect(isAllowedCl8yProductHref(link.href)).toBe(true)
      expect(link.label.toLowerCase()).not.toMatch(/security|report|connect wallet/)
      expect(link.href).not.toContain('dex.cl8y.com')
      expect(link.href).not.toMatch(/telegram|twitter|x\.com|gamefi/i)
    }
  })
})
