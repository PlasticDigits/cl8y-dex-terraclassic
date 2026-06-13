/** Hostnames allowed for remote token logo `<img src>` (GitLab #378 / M-09). */
const TRUSTED_LOGO_HOSTS = new Set([
  'gitlab.com',
  'raw.githubusercontent.com',
  'assets.coingecko.com',
  'ipfs.io',
  'cloudflare-ipfs.com',
])

export function isTrustedTokenLogoUrl(url: string): boolean {
  try {
    const parsed = new URL(url)
    if (parsed.protocol !== 'https:') return false
    return TRUSTED_LOGO_HOSTS.has(parsed.hostname)
  } catch {
    return false
  }
}

/** Returns the URI when the host is allowlisted; otherwise `undefined` (caller falls back to blockie). */
export function resolveTrustedTokenLogoUrl(url: string | undefined | null): string | undefined {
  const trimmed = url?.trim()
  if (!trimmed) return undefined
  return isTrustedTokenLogoUrl(trimmed) ? trimmed : undefined
}
