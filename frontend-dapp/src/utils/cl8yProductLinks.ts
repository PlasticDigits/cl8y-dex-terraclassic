/**
 * Frozen official CL8Y sibling-product URLs for the dApp footer (GitLab #663).
 *
 * Compile-time constants only — never `import.meta.env.VITE_*`, searchParams, hash,
 * or indexer fields. Header More / Swap banners must not import this list.
 *
 * Invariants: docs/frontend.md § Official CL8Y product links; P663-1–P663-8.
 */

export const CL8Y_PRODUCT_HOME_HREF = 'https://cl8y.com/' as const
export const CL8Y_PRODUCT_BRIDGE_HREF = 'https://bridge.cl8y.com/' as const

export const CL8Y_PRODUCT_LINKS = [
  {
    id: 'home',
    label: 'Homepage',
    href: CL8Y_PRODUCT_HOME_HREF,
    testId: 'footer-product-home',
  },
  {
    id: 'bridge',
    label: 'Bridge',
    href: CL8Y_PRODUCT_BRIDGE_HREF,
    testId: 'footer-product-bridge',
  },
] as const

export type Cl8yProductLink = (typeof CL8Y_PRODUCT_LINKS)[number]

const ALLOWED_HOSTS = new Set(['cl8y.com', 'bridge.cl8y.com'])

/**
 * True only for the two pinned HTTPS origins (trailing slash optional).
 * Rejects http, protocol-relative, javascript/data, userinfo, ports, query/hash,
 * extra paths, lookalike hosts, and `dex.cl8y.com` (the user is already here).
 */
export function isAllowedCl8yProductHref(href: string): boolean {
  if (typeof href !== 'string' || href.trim() === '') return false
  const trimmed = href.trim()
  if (trimmed.startsWith('//') || trimmed.startsWith('\\')) return false

  let parsed: URL
  try {
    parsed = new URL(trimmed)
  } catch {
    return false
  }

  if (parsed.protocol !== 'https:') return false
  if (parsed.username !== '' || parsed.password !== '') return false
  if (parsed.port !== '') return false
  if (parsed.search !== '' || parsed.hash !== '') return false
  if (!ALLOWED_HOSTS.has(parsed.hostname)) return false
  if (parsed.pathname !== '/' && parsed.pathname !== '') return false
  return true
}

/** Canonical allowlist href, or `null` when `href` is not one of the two pins. */
export function canonicalCl8yProductHref(href: string): string | null {
  if (!isAllowedCl8yProductHref(href)) return null
  return new URL(href.trim()).hostname === 'bridge.cl8y.com' ? CL8Y_PRODUCT_BRIDGE_HREF : CL8Y_PRODUCT_HOME_HREF
}
