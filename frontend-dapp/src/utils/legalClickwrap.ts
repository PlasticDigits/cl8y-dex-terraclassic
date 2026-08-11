/**
 * CL8Y Legal clickwrap wiring for the DEX (GitLab #517).
 *
 * Signing happens on the hosted portal; this app only checks status and redirects.
 * Do not reimplement Terra Classic wallet signature verify here.
 */
import {
  createClient,
  isAllowedRedirectUri,
  sanitizeRedirectUri,
  type ClickwrapClient,
} from '@plasticdigits/cl8y-clickwrap'

/** Product property registered on the Legal API (hostname-scoped acceptances). */
export const DEFAULT_LEGAL_PROPERTY = 'dex.cl8y.com'

export const DEFAULT_LEGAL_API_BASE_URL = 'https://api.terms.cl8y.com'
export const DEFAULT_LEGAL_TERMS_BASE_URL = 'https://terms.cl8y.com'

/** Origins the DEX may pass as portal `redirect_uri` (portal still enforces its allowlist). */
export const LEGAL_REDIRECT_ALLOWLIST = ['https://dex.cl8y.com'] as const

let clientSingleton: ClickwrapClient | null = null

export function getLegalProperty(): string {
  const fromEnv = import.meta.env.VITE_LEGAL_PROPERTY?.trim()
  return fromEnv || DEFAULT_LEGAL_PROPERTY
}

export function getLegalApiBaseUrl(): string {
  const fromEnv = import.meta.env.VITE_LEGAL_API_BASE_URL?.trim()
  return (fromEnv || DEFAULT_LEGAL_API_BASE_URL).replace(/\/$/, '')
}

export function getLegalTermsBaseUrl(): string {
  const fromEnv = import.meta.env.VITE_LEGAL_TERMS_BASE_URL?.trim()
  return (fromEnv || DEFAULT_LEGAL_TERMS_BASE_URL).replace(/\/$/, '')
}

export function getLegalClickwrapClient(): ClickwrapClient {
  if (!clientSingleton) {
    clientSingleton = createClient({
      apiBaseUrl: getLegalApiBaseUrl(),
      termsBaseUrl: getLegalTermsBaseUrl(),
    })
  }
  return clientSingleton
}

/** Test helper — clears the memoized client between cases. */
export function resetLegalClickwrapClientForTests(): void {
  clientSingleton = null
}

/**
 * Playwright webServer sets `VITE_PLAYWRIGHT_E2E=true` so E2E is not blocked by Legal
 * (same escape hatch as the first-visit risk modal — GitLab #138 / #517).
 * Never enable on production / manual soft-launch builds.
 */
export function skipLegalClickwrapForAutomation(): boolean {
  return import.meta.env.VITE_PLAYWRIGHT_E2E === 'true'
}

export function allowLegalLocalhostRedirect(): boolean {
  // Prod bundles must not send loopback redirect_uri to the portal.
  return !import.meta.env.PROD
}

export function getLegalRedirectAllowlist(): string[] {
  const extra = import.meta.env.VITE_LEGAL_REDIRECT_ALLOWLIST?.trim()
  if (!extra) return [...LEGAL_REDIRECT_ALLOWLIST]
  const parsed = extra
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
  return [...new Set([...LEGAL_REDIRECT_ALLOWLIST, ...parsed])]
}

/**
 * Fail-fast sanitize of `window.location.href` before portal redirect.
 * Portal allowlist remains the source of truth.
 */
export function resolveLegalRedirectUri(href?: string): string | null {
  const candidate = href ?? (typeof window !== 'undefined' ? window.location.href : '')
  if (!candidate) return null
  return sanitizeRedirectUri(candidate, {
    allowlist: getLegalRedirectAllowlist(),
    allowLocalhost: allowLegalLocalhostRedirect(),
  })
}

export function isLegalRedirectUriAllowed(uri: string): boolean {
  return isAllowedRedirectUri(uri, {
    allowlist: getLegalRedirectAllowlist(),
    allowLocalhost: allowLegalLocalhostRedirect(),
  })
}
