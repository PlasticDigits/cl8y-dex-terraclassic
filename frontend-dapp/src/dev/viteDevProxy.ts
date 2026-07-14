/**
 * Vite same-origin proxies for remote LCD / indexer during local `vite` serve.
 * Avoids browser CORS when pointing `.env.local` at soft-launch / mainnet hosts
 * (production indexer CORS only allows `https://dex.cl8y.com`).
 *
 * Browser sees: `/__dev/indexer/*` and `/__dev/lcd/*`
 * Vite forwards to: VITE_INDEXER_URL / VITE_TERRA_LCD_URL upstreams
 */

export const DEV_PROXY_INDEXER_PREFIX = '/__dev/indexer'
export const DEV_PROXY_LCD_PREFIX = '/__dev/lcd'

export function isLoopbackHostname(hostname: string): boolean {
  const h = hostname.toLowerCase()
  return h === 'localhost' || h === '127.0.0.1' || h === '[::1]' || h === '::1'
}

/** Absolute http(s) URL whose host is not loopback (needs CORS or a Vite proxy). */
export function isRemoteHttpUrl(raw: string | undefined): boolean {
  const trimmed = raw?.trim()
  if (!trimmed) return false
  try {
    const url = new URL(trimmed)
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return false
    return !isLoopbackHostname(url.hostname)
  } catch {
    return false
  }
}

export function stripTrailingSlash(url: string): string {
  return url.replace(/\/+$/, '')
}

export type DevRemoteProxyPlan = {
  /** Upstream absolute URL for indexer proxy target (no trailing slash). */
  indexerTarget: string | null
  /** Upstream absolute URL for LCD proxy target (no trailing slash). */
  lcdTarget: string | null
  /** Client-facing VITE_INDEXER_URL override (same-origin path). */
  indexerBrowserUrl: string | null
  /** Client-facing VITE_TERRA_LCD_URL override (same-origin path). */
  lcdBrowserUrl: string | null
}

/**
 * Decide proxy plan for Vite `server.proxy` + `define` overrides.
 * Enabled when `VITE_DEV_PROXY=1` / `true`, or when either URL is remote http(s)
 * and `VITE_DEV_PROXY` is not explicitly `0` / `false`.
 */
export function planDevRemoteProxy(env: Record<string, string>): DevRemoteProxyPlan {
  const flag = (env.VITE_DEV_PROXY || '').trim().toLowerCase()
  const forcedOff = flag === '0' || flag === 'false' || flag === 'off'
  const forcedOn = flag === '1' || flag === 'true' || flag === 'on'

  const indexerRaw = env.VITE_INDEXER_URL?.trim()
  const lcdRaw = env.VITE_TERRA_LCD_URL?.trim()
  const indexerRemote = isRemoteHttpUrl(indexerRaw)
  const lcdRemote = isRemoteHttpUrl(lcdRaw)

  if (forcedOff || (!forcedOn && !indexerRemote && !lcdRemote)) {
    return { indexerTarget: null, lcdTarget: null, indexerBrowserUrl: null, lcdBrowserUrl: null }
  }

  return {
    indexerTarget: indexerRemote && indexerRaw ? stripTrailingSlash(indexerRaw) : null,
    lcdTarget: lcdRemote && lcdRaw ? stripTrailingSlash(lcdRaw) : null,
    indexerBrowserUrl: indexerRemote ? DEV_PROXY_INDEXER_PREFIX : null,
    lcdBrowserUrl: lcdRemote ? DEV_PROXY_LCD_PREFIX : null,
  }
}
