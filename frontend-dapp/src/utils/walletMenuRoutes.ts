import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'

/** App-route for the connected-menu Portfolio row (GitLab #185 / #671). */
export const WALLET_PORTFOLIO_PATH = '/portfolio' as const

/**
 * Same-origin trader profile path. Returns `null` when `address` is not a valid
 * `terra` bech32 so `/trader/${address}` cannot become `//host` or a scheme URL (#671).
 */
export function traderProfilePath(address: string): string | null {
  const trimmed = address.trim()
  if (!trimmed || !isValidTerraBech32Address(trimmed)) return null
  return `/trader/${trimmed}`
}
