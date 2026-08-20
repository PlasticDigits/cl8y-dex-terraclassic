/**
 * Build-time Open Graph / Twitter card origin (GitLab #578).
 *
 * Production `index.html` must ship absolute `https://` image URLs. Origins are
 * allowlisted here and baked by Vite — never from the request host header,
 * the browser location object, query, hash, pair address, or wallet address.
 */

export const DEFAULT_PUBLIC_ORIGIN = 'https://dex.cl8y.com'
export const OG_IMAGE_PATH = '/og-image.png'
export const OG_IMAGE_WIDTH = 1200
export const OG_IMAGE_HEIGHT = 630
export const OG_SITE_NAME = 'CL8Y DEX'
export const OG_IMAGE_ALT = 'CL8Y DEX medallion with laurel portrait, scales, and a rising market chart'
export const OG_IMAGE_TYPE = 'image/png'
export const TWITTER_CARD = 'summary_large_image'

/** Explicit https origins that may be baked into OG tags. Add staging hosts here. */
export const PUBLIC_ORIGIN_ALLOWLIST = [DEFAULT_PUBLIC_ORIGIN] as const

export class PublicOriginError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'PublicOriginError'
  }
}

function isAllowlistedOrigin(origin: string): boolean {
  return (PUBLIC_ORIGIN_ALLOWLIST as readonly string[]).includes(origin)
}

/**
 * Parse and allowlist a public origin for OG/Twitter tags.
 * Rejects `http:`, `javascript:`, `data:`, protocol-relative, credentials, query, and hash.
 */
export function resolvePublicOrigin(raw: string | undefined): string {
  const trimmed = raw?.trim()
  if (!trimmed) return DEFAULT_PUBLIC_ORIGIN

  if (trimmed.startsWith('//') || trimmed.startsWith('\\\\')) {
    throw new PublicOriginError('VITE_PUBLIC_ORIGIN must not be protocol-relative')
  }

  let url: URL
  try {
    url = new URL(trimmed)
  } catch {
    throw new PublicOriginError('VITE_PUBLIC_ORIGIN is not a valid URL')
  }

  if (url.protocol !== 'https:') {
    throw new PublicOriginError('VITE_PUBLIC_ORIGIN must be https://')
  }
  if (url.username || url.password) {
    throw new PublicOriginError('VITE_PUBLIC_ORIGIN must not include credentials')
  }
  if (url.search || url.hash) {
    throw new PublicOriginError('VITE_PUBLIC_ORIGIN must not include query or hash')
  }
  if (url.pathname !== '/' && url.pathname !== '') {
    throw new PublicOriginError('VITE_PUBLIC_ORIGIN must not include a path')
  }

  const origin = url.origin
  if (!isAllowlistedOrigin(origin)) {
    throw new PublicOriginError(
      `VITE_PUBLIC_ORIGIN ${origin} is not allowlisted (GitLab #578). ` +
        `Add it to PUBLIC_ORIGIN_ALLOWLIST in viteOg.ts after review.`
    )
  }
  return origin
}

function metaContentPattern(attr: 'property' | 'name', key: string): RegExp {
  return new RegExp(`<meta\\s+${attr}="${key}"\\s+content="[^"]*"\\s*/>`)
}

function metaTag(attr: 'property' | 'name', key: string, content: string): string {
  return `<meta ${attr}="${key}" content="${content}" />`
}

function replaceMetaContent(html: string, attr: 'property' | 'name', key: string, content: string): string {
  const pattern = metaContentPattern(attr, key)
  if (!pattern.test(html)) {
    throw new PublicOriginError(`index.html is missing ${attr}="${key}"`)
  }
  return html.replace(pattern, metaTag(attr, key, content))
}

function upsertMeta(html: string, attr: 'property' | 'name', key: string, content: string, afterKey: string): string {
  const pattern = metaContentPattern(attr, key)
  if (pattern.test(html)) {
    return html.replace(pattern, metaTag(attr, key, content))
  }
  const after = metaContentPattern('property', afterKey)
  if (!after.test(html)) {
    throw new PublicOriginError(`index.html is missing property="${afterKey}" (insert point for ${key})`)
  }
  return html.replace(after, (match) => `${match}\n    ${metaTag(attr, key, content)}`)
}

/** Rewrite relative OG/Twitter image URLs to an allowlisted absolute origin. */
export function bakeProductionOgHtml(html: string, origin: string): string {
  const safeOrigin = resolvePublicOrigin(origin)
  const imageUrl = `${safeOrigin}${OG_IMAGE_PATH}`
  const pageUrl = `${safeOrigin}/`

  let out = replaceMetaContent(html, 'property', 'og:image', imageUrl)
  out = replaceMetaContent(out, 'name', 'twitter:image', imageUrl)
  out = upsertMeta(out, 'property', 'og:image:secure_url', imageUrl, 'og:image')
  out = upsertMeta(out, 'property', 'og:url', pageUrl, 'og:type')
  out = upsertMeta(out, 'property', 'og:site_name', OG_SITE_NAME, 'og:type')

  if (out.includes('content="/og-image.png"')) {
    throw new PublicOriginError('production OG bake left a relative /og-image.png')
  }
  if (/content="\/\//.test(out) || /content="javascript:/i.test(out) || /content="data:/i.test(out)) {
    throw new PublicOriginError('production OG bake produced a non-https URL')
  }
  return out
}
