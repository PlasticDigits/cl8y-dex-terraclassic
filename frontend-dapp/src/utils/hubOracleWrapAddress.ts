import { LUNC_C_TOKEN_ADDRESS, USTC_C_TOKEN_ADDRESS } from '@/utils/constants'
import { isHubOracleWrapTicker, type HubPriceTicker } from '@/utils/hubPriceTicker'
import { getExplorerAddressUrl } from '@/utils/terraExplorer'

/**
 * Wrap CW20 for oracle-anchored hub columns (cUSTC, LUNC / cLUNC).
 * Configured env overlay wins. Invalid / empty → omit the AddressRow (GitLab #570).
 * Explorer hrefs still go through `getExplorerAddressUrl` — never trust indexer JSON.
 */
export function resolveHubOracleWrapAddress(ticker: HubPriceTicker, apiAssetAddress?: string | null): string | null {
  if (!isHubOracleWrapTicker(ticker)) return null
  const envRaw = ticker === 'custc' ? USTC_C_TOKEN_ADDRESS : LUNC_C_TOKEN_ADDRESS
  const env = typeof envRaw === 'string' ? envRaw.trim() : ''
  if (env) {
    return getExplorerAddressUrl(env) ? env : null
  }
  const api = typeof apiAssetAddress === 'string' ? apiAssetAddress.trim() : ''
  if (!api) return null
  return getExplorerAddressUrl(api) ? api : null
}
