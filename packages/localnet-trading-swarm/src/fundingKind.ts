/** LocalTerra CW20 funding fork (GitLab #620). */

export type Cw20FundingKind = 'skip' | 'transfer' | 'mint'

export interface Cw20FundingEnv {
  wrapAddresses: string[]
  taxTokenAddress?: string
}

export function fundingEnvFromVite(
  env: Record<string, string | undefined>
): Cw20FundingEnv {
  const wrap: string[] = []
  if (env.VITE_LUNC_C_TOKEN_ADDRESS) wrap.push(env.VITE_LUNC_C_TOKEN_ADDRESS)
  if (env.VITE_USTC_C_TOKEN_ADDRESS) wrap.push(env.VITE_USTC_C_TOKEN_ADDRESS)
  const tax = env.VITE_TOKEN_COMMUNITY_TAX_ADDRESS
  return {
    wrapAddresses: wrap,
    taxTokenAddress: tax && tax.length > 0 ? tax : undefined,
  }
}

/**
 * Sync classify. `originLauncher` is `GetLauncherOrigin.launcher` when already
 * queried; omit it to use the env pin only.
 */
export function classifyCw20FundingKind(
  token: string,
  env: Cw20FundingEnv,
  originLauncher?: string | null
): Cw20FundingKind {
  if (env.wrapAddresses.includes(token)) return 'skip'
  if (env.taxTokenAddress && token === env.taxTokenAddress) return 'transfer'
  if (originLauncher) return 'transfer'
  return 'mint'
}
