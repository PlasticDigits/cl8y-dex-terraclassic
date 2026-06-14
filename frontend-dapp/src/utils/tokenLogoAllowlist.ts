/** Hostnames allowed for remote token logo `<img src>` (GitLab #378 / M-09). */
export const TOKEN_LOGO_ALLOWED_HOSTS = [
  'gitlab.com',
  'raw.githubusercontent.com',
  'assets.coingecko.com',
  'coin-images.coingecko.com',
  's2.coinmarketcap.com',
  'static.coinmarketcap.com',
  'ipfs.io',
  'cloudflare-ipfs.com',
] as const

export function isAllowedTokenLogoHost(hostname: string): boolean {
  const host = hostname.toLowerCase()
  return TOKEN_LOGO_ALLOWED_HOSTS.some((allowed) => host === allowed || host.endsWith(`.${allowed}`))
}

/** Returns the URI when https and host is allowlisted; otherwise undefined (fall back to blockie). */
export function resolveAllowedTokenLogoUri(uri: string | undefined | null): string | undefined {
  if (!uri?.trim()) return undefined
  try {
    const parsed = new URL(uri.trim())
    if (parsed.protocol !== 'https:') return undefined
    if (!isAllowedTokenLogoHost(parsed.hostname)) return undefined
    return parsed.href
  } catch {
    return undefined
  }
}

/** @deprecated Use `resolveAllowedTokenLogoUri` — kept for callers merged from main. */
export const resolveTrustedTokenLogoUrl = resolveAllowedTokenLogoUri

/** @deprecated Use `isAllowedTokenLogoHost` — kept for callers merged from main. */
export function isTrustedTokenLogoUrl(url: string): boolean {
  return resolveAllowedTokenLogoUri(url) !== undefined
}
