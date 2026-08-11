import { TOKEN_LOGO_ALLOWED_HOSTS } from './src/utils/tokenLogoAllowlist'

const WALLETCONNECT_CONNECT_HOSTS = [
  'wss://relay.walletconnect.com',
  'wss://relay.walletconnect.org',
  'https://verify.walletconnect.com',
  'https://explorer-api.walletconnect.com',
  'https://pulse.walletconnect.org',
] as const

/** Mirrors `TERRA_LCD_URL` / `TERRA_RPC_URL` fallbacks in `src/utils/constants.ts`. */
export const PRODUCTION_TERRA_LCD_FALLBACK = 'https://terra-classic-lcd.publicnode.com'
export const PRODUCTION_TERRA_RPC_FALLBACK = 'https://terra-classic-rpc.publicnode.com:443'

/** Legal API + portal defaults for production connect-src (GitLab #517). Keep in sync with `legalClickwrap.ts`. */
export const PRODUCTION_LEGAL_API_ORIGIN = 'https://api.terms.cl8y.com'
export const PRODUCTION_LEGAL_TERMS_ORIGIN = 'https://terms.cl8y.com'

function originFromEnvUrl(raw: string | undefined): string | null {
  const trimmed = raw?.trim()
  if (!trimmed) return null
  try {
    const url = new URL(trimmed)
    return `${url.protocol}//${url.host}`
  } catch {
    return null
  }
}

function uniqueHosts(hosts: Array<string | null | undefined>): string[] {
  return [...new Set(hosts.filter((h): h is string => Boolean(h)))]
}

/** Production connect-src: env LCD/RPC/indexer + WalletConnect + Legal API (GitLab #378 / M-07 / #517). */
export function buildProductionConnectSrc(env: Record<string, string>): string {
  const fromEnv = uniqueHosts([
    originFromEnvUrl(env.VITE_TERRA_LCD_URL) ?? PRODUCTION_TERRA_LCD_FALLBACK,
    originFromEnvUrl(env.VITE_TERRA_RPC_URL) ?? PRODUCTION_TERRA_RPC_FALLBACK,
    originFromEnvUrl(env.VITE_INDEXER_URL),
    ...WALLETCONNECT_CONNECT_HOSTS,
    originFromEnvUrl(env.VITE_LEGAL_API_BASE_URL) ?? PRODUCTION_LEGAL_API_ORIGIN,
    originFromEnvUrl(env.VITE_LEGAL_TERMS_BASE_URL) ?? PRODUCTION_LEGAL_TERMS_ORIGIN,
  ])
  return ["'self'", ...fromEnv].join(' ')
}

/** Production img-src https hosts mirror the token logo allowlist. */
export function buildProductionImgSrc(): string {
  const hosts = TOKEN_LOGO_ALLOWED_HOSTS.map((host) => `https://${host}`)
  return ["'self'", 'data:', ...hosts].join(' ')
}

export function buildProductionCspMetaContent(env: Record<string, string>): string {
  return [
    "default-src 'self'",
    "script-src 'self' 'unsafe-inline'",
    "style-src 'self' 'unsafe-inline'",
    `connect-src ${buildProductionConnectSrc(env)}`,
    `img-src ${buildProductionImgSrc()}`,
    "font-src 'self'",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
    "frame-ancestors 'none'",
  ].join('; ')
}

export const DEV_CSP_META_CONTENT =
  "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self' https: wss: http://localhost:* http://127.0.0.1:*; img-src 'self' data: https:; font-src 'self';"
