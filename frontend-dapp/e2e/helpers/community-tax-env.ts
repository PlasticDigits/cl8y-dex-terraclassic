/**
 * Community-tax Vite pins for Playwright (GitLab #602 smoke / #622 tx).
 *
 * **Smoke** (`e2e-smoke`, `/token/create` chrome): columbus-5 **11611** + launcher
 * bake so the page is not the unavailable stub when `DEPLOY_SKIP_COMMUNITY_TAX=1`.
 *
 * **Tx** (`e2e-tx`, `community-tax-tx.spec.ts`): read `frontend-dapp/.env.local`
 * only. Columbus-5 code ids / launcher fail closed (**E622-2**, **E622-3**).
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  communityTaxViteEnvForPlaywright,
  MAINNET_COMMUNITY_TAX_CODE_ID,
  MAINNET_COMMUNITY_TOKEN_LAUNCHER,
  parseCommunityTaxTxPins,
  type CommunityTaxTxPins,
} from '../../src/utils/communityTaxTxEnv'

export { MAINNET_COMMUNITY_TAX_CODE_ID, MAINNET_COMMUNITY_TOKEN_LAUNCHER }
export type { CommunityTaxTxPins }

const here = path.dirname(fileURLToPath(import.meta.url))

export function readFrontendEnvLocal(): Record<string, string> {
  const envLocal = path.join(here, '..', '..', '.env.local')
  const out: Record<string, string> = {}
  if (!fs.existsSync(envLocal)) return out
  for (const line of fs.readFileSync(envLocal, 'utf8').split('\n')) {
    const m = line.match(/^(VITE_[A-Z0-9_]+)=(.*)$/)
    if (!m) continue
    out[m[1]] = m[2].trim().replace(/^["']|["']$/g, '')
  }
  return out
}

function envOrLocal(local: Record<string, string>, key: string): string | undefined {
  const fromProcess = process.env[key]?.trim()
  if (fromProcess) return fromProcess
  return local[key]
}

const envLocalSnapshot = readFrontendEnvLocal()

/**
 * Playwright `webServer.env` bake.
 * Prefers LocalTerra `.env.local` so extra-debit Max matches the seed token
 * `code_id`. Falls back to columbus-5 only when local pins are absent (smoke).
 */
export const COMMUNITY_TAX_E2E_VITE_ENV = communityTaxViteEnvForPlaywright({
  VITE_COMMUNITY_TAX_CODE_ID: envOrLocal(envLocalSnapshot, 'VITE_COMMUNITY_TAX_CODE_ID'),
  VITE_COMMUNITY_TOKEN_LAUNCHER: envOrLocal(envLocalSnapshot, 'VITE_COMMUNITY_TOKEN_LAUNCHER'),
})

/** Strict e2e-tx pins. Never `test.skip` — missing seed is a failed run (**E622-2**). */
export function requireCommunityTaxTxPins(): CommunityTaxTxPins {
  const local = readFrontendEnvLocal()
  return parseCommunityTaxTxPins({
    VITE_TOKEN_COMMUNITY_TAX_ADDRESS: envOrLocal(local, 'VITE_TOKEN_COMMUNITY_TAX_ADDRESS'),
    VITE_PAIR_COMMUNITY_TAX_EMBER: envOrLocal(local, 'VITE_PAIR_COMMUNITY_TAX_EMBER'),
    VITE_TOKEN_EMBER_ADDRESS: envOrLocal(local, 'VITE_TOKEN_EMBER_ADDRESS'),
    VITE_COMMUNITY_TOKEN_LAUNCHER: envOrLocal(local, 'VITE_COMMUNITY_TOKEN_LAUNCHER'),
    VITE_COMMUNITY_TAX_CODE_ID: envOrLocal(local, 'VITE_COMMUNITY_TAX_CODE_ID'),
    VITE_TOKEN_COMMUNITY_TAX_SYMBOL: envOrLocal(local, 'VITE_TOKEN_COMMUNITY_TAX_SYMBOL'),
  })
}
