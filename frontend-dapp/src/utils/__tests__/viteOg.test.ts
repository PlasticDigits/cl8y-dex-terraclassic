/** @vitest-environment node */
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import {
  bakeProductionOgHtml,
  DEFAULT_PUBLIC_ORIGIN,
  OG_IMAGE_ALT,
  OG_IMAGE_HEIGHT,
  OG_IMAGE_PATH,
  OG_IMAGE_TYPE,
  OG_IMAGE_WIDTH,
  OG_SITE_NAME,
  PublicOriginError,
  resolvePublicOrigin,
  TWITTER_CARD,
} from '../../../viteOg'

const frontendRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..')
const indexHtml = readFileSync(path.join(frontendRoot, 'index.html'), 'utf8')

describe('resolvePublicOrigin (GitLab #578)', () => {
  it('defaults production origin to dex.cl8y.com', () => {
    expect(resolvePublicOrigin(undefined)).toBe(DEFAULT_PUBLIC_ORIGIN)
    expect(resolvePublicOrigin('')).toBe(DEFAULT_PUBLIC_ORIGIN)
    expect(resolvePublicOrigin('   ')).toBe(DEFAULT_PUBLIC_ORIGIN)
  })

  it('accepts the allowlisted production origin', () => {
    expect(resolvePublicOrigin('https://dex.cl8y.com')).toBe(DEFAULT_PUBLIC_ORIGIN)
    expect(resolvePublicOrigin('https://dex.cl8y.com/')).toBe(DEFAULT_PUBLIC_ORIGIN)
  })

  it('rejects http, javascript, data, and protocol-relative origins', () => {
    expect(() => resolvePublicOrigin('http://dex.cl8y.com')).toThrow(PublicOriginError)
    expect(() => resolvePublicOrigin('javascript:alert(1)')).toThrow(PublicOriginError)
    expect(() => resolvePublicOrigin('data:text/html,x')).toThrow(PublicOriginError)
    expect(() => resolvePublicOrigin('//dex.cl8y.com')).toThrow(PublicOriginError)
    expect(() => resolvePublicOrigin('https://evil.example')).toThrow(PublicOriginError)
    expect(() => resolvePublicOrigin('https://dex.cl8y.com/?og=https://evil/x.png')).toThrow(PublicOriginError)
    expect(() => resolvePublicOrigin('https://user:pass@dex.cl8y.com')).toThrow(PublicOriginError)
  })
})

describe('bakeProductionOgHtml (GitLab #578)', () => {
  it('rewrites relative OG and Twitter image URLs to absolute https', () => {
    const baked = bakeProductionOgHtml(indexHtml, DEFAULT_PUBLIC_ORIGIN)
    const image = `${DEFAULT_PUBLIC_ORIGIN}${OG_IMAGE_PATH}`
    expect(baked).toContain(`property="og:image" content="${image}"`)
    expect(baked).toContain(`property="og:image:secure_url" content="${image}"`)
    expect(baked).toContain(`name="twitter:image" content="${image}"`)
    expect(baked).toContain(`property="og:url" content="${DEFAULT_PUBLIC_ORIGIN}/"`)
    expect(baked).toContain(`property="og:site_name" content="${OG_SITE_NAME}"`)
    expect(baked).not.toContain('content="/og-image.png"')
    expect(baked).not.toMatch(/content="\/\//)
  })

  it('keeps summary_large_image and product copy', () => {
    const baked = bakeProductionOgHtml(indexHtml, DEFAULT_PUBLIC_ORIGIN)
    expect(baked).toContain(`name="twitter:card" content="${TWITTER_CARD}"`)
    expect(baked).toContain('Swap & Limit Orders on Terra Classic')
    expect(baked).toContain('Easy swaps, limit orders')
    expect(baked).not.toContain('name="twitter:site"')
  })

  it('keeps image dimension and alt tags matching the shipped file', () => {
    expect(indexHtml).toContain(`property="og:image:type" content="${OG_IMAGE_TYPE}"`)
    expect(indexHtml).toContain(`property="og:image:height" content="${OG_IMAGE_HEIGHT}"`)
    expect(indexHtml).toContain(`property="og:image:alt" content="${OG_IMAGE_ALT}"`)
    expect(indexHtml).toContain(`name="twitter:image:alt" content="${OG_IMAGE_ALT}"`)
    const baked = bakeProductionOgHtml(indexHtml, DEFAULT_PUBLIC_ORIGIN)
    expect(baked).toContain(`property="og:image:width" content="${OG_IMAGE_WIDTH}"`)
    expect(baked).toContain(`property="og:image:height" content="${OG_IMAGE_HEIGHT}"`)
    expect(baked).toContain(OG_IMAGE_ALT)
  })

  it('does not interpolate query, hash, pair, or wallet into meta', () => {
    const baked = bakeProductionOgHtml(indexHtml, DEFAULT_PUBLIC_ORIGIN)
    expect(baked).not.toMatch(/og:image[^>]+content="[^"]*\?/)
    expect(baked).not.toMatch(/terra1/)
    expect(baked).not.toContain('evil.example')
  })
})
