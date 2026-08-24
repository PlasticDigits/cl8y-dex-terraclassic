/**
 * LocalTerra tax-pair pins for Playwright e2e-tx (GitLab #622).
 *
 * Smoke `/token/create` chrome may bake columbus-5 launcher / **11611**.
 * Tx specs must use seed-deploy `.env.local` pins — never columbus-5.
 */

/** Columbus-5 community-tax / launcher / AutoLP / ALPHA stores. Local seed must not use these. */
export const COLUMBUS5_COMMUNITY_TAX_CODE_IDS = [11611, 11612, 11613, 11614, 11619, 11620, 11621, 11622, 8654] as const

export const MAINNET_COMMUNITY_TAX_CODE_ID = '11611'
export const MAINNET_COMMUNITY_TOKEN_LAUNCHER = 'terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze'

export type CommunityTaxTxPins = {
  token: string
  pair: string
  ember: string
  launcher: string
  codeId: string
  symbol: string
}

const TERRA1 = /^terra1[0-9a-z]{38,}$/

export function isColumbus5CommunityTaxCodeId(codeId: string | undefined): boolean {
  const n = Number.parseInt((codeId ?? '').trim(), 10)
  if (!Number.isFinite(n)) return false
  return (COLUMBUS5_COMMUNITY_TAX_CODE_IDS as readonly number[]).includes(n)
}

export function isTerraContractAddr(value: string | undefined): value is string {
  return !!value && TERRA1.test(value.trim())
}

/** Fail-closed: local seed pins only. Columbus-5 bake-in is smoke-only. */
export function parseCommunityTaxTxPins(env: Record<string, string | undefined>): CommunityTaxTxPins {
  const token = env.VITE_TOKEN_COMMUNITY_TAX_ADDRESS?.trim()
  const pair = env.VITE_PAIR_COMMUNITY_TAX_EMBER?.trim()
  const ember = env.VITE_TOKEN_EMBER_ADDRESS?.trim()
  const launcher = env.VITE_COMMUNITY_TOKEN_LAUNCHER?.trim()
  const codeId = env.VITE_COMMUNITY_TAX_CODE_ID?.trim()
  const symbol = (env.VITE_TOKEN_COMMUNITY_TAX_SYMBOL?.trim() || 'QTAX').toUpperCase()

  if (!isTerraContractAddr(token) || !isTerraContractAddr(pair) || !isTerraContractAddr(ember)) {
    throw new Error(
      'Community-tax e2e-tx needs VITE_TOKEN_COMMUNITY_TAX_ADDRESS, VITE_PAIR_COMMUNITY_TAX_EMBER, and VITE_TOKEN_EMBER_ADDRESS in frontend-dapp/.env.local (make deploy-local without DEPLOY_SKIP_COMMUNITY_TAX). GitLab #622 / E622-2.'
    )
  }
  if (!isTerraContractAddr(launcher)) {
    throw new Error(
      'Community-tax e2e-tx needs local VITE_COMMUNITY_TOKEN_LAUNCHER in frontend-dapp/.env.local (GitLab #622 / E622-2).'
    )
  }
  if (launcher === MAINNET_COMMUNITY_TOKEN_LAUNCHER) {
    throw new Error(
      'Community-tax e2e-tx refuses columbus-5 launcher terra126pr5…ahzwze. Use the LocalTerra seed pin (GitLab #622 / E622-3).'
    )
  }
  if (!codeId || isColumbus5CommunityTaxCodeId(codeId)) {
    throw new Error(
      `Community-tax e2e-tx refuses columbus-5 / forbidden code id ${codeId ?? '(missing)'}. Seed deploy writes a local store id (GitLab #622 / E622-3).`
    )
  }
  return { token, pair, ember, launcher, codeId, symbol }
}

/** Vite env for Playwright webServer: local pins when present, else smoke columbus-5 fallback. */
export function communityTaxViteEnvForPlaywright(env: Record<string, string | undefined>): {
  VITE_COMMUNITY_TAX_CODE_ID: string
  VITE_COMMUNITY_TOKEN_LAUNCHER: string
} {
  const code = env.VITE_COMMUNITY_TAX_CODE_ID?.trim()
  const launcher = env.VITE_COMMUNITY_TOKEN_LAUNCHER?.trim()
  if (code && launcher && !isColumbus5CommunityTaxCodeId(code) && launcher !== MAINNET_COMMUNITY_TOKEN_LAUNCHER) {
    return {
      VITE_COMMUNITY_TAX_CODE_ID: code,
      VITE_COMMUNITY_TOKEN_LAUNCHER: launcher,
    }
  }
  return {
    VITE_COMMUNITY_TAX_CODE_ID: env.VITE_COMMUNITY_TAX_CODE_ID?.trim() || MAINNET_COMMUNITY_TAX_CODE_ID,
    VITE_COMMUNITY_TOKEN_LAUNCHER: env.VITE_COMMUNITY_TOKEN_LAUNCHER?.trim() || MAINNET_COMMUNITY_TOKEN_LAUNCHER,
  }
}
