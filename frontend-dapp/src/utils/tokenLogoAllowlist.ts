/** HTTPS hosts allowed for remote token logos (GitLab #378 / M-09). */
const ALLOWED_LOGO_HOSTS = new Set([
  'gitlab.com',
  'raw.githubusercontent.com',
  'assets.coingecko.com',
  'coin-images.coingecko.com',
  's2.coinmarketcap.com',
])

export function isAllowedLogoUrl(url: string): boolean {
  try {
    const parsed = new URL(url.trim())
    if (parsed.protocol !== 'https:') return false
    return ALLOWED_LOGO_HOSTS.has(parsed.hostname)
  } catch {
    return false
  }
}

/** Returns the URI when the host is allowlisted; otherwise undefined (caller falls back to blockie). */
export function resolveSafeLogoUrl(logoURI?: string): string | undefined {
  if (!logoURI?.trim()) return undefined
  const trimmed = logoURI.trim()
  return isAllowedLogoUrl(trimmed) ? trimmed : undefined
}
