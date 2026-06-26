import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import type { Page, Route } from '@playwright/test'

export interface BlacklistCheckLcdResponse {
  blocked: boolean
  wallet_blacklisted: boolean
  blacklisted_tokens: string[]
  pair_blacklisted: boolean
  blacklisted_pairs: string[]
}

function repoRootFromE2e(): string {
  const here = path.dirname(fileURLToPath(import.meta.url))
  return path.resolve(here, '..', '..', '..')
}

function decodeSmartQuery(url: string): { contract: string; query: Record<string, unknown> } | null {
  const match = url.match(/\/contract\/([^/]+)\/smart\/([^/?#]+)/)
  if (!match) return null
  const contract = match[1]
  const seg = decodeURIComponent(match[2])
  try {
    const query = JSON.parse(Buffer.from(seg, 'base64').toString('utf8')) as Record<string, unknown>
    return { contract, query }
  } catch {
    return null
  }
}

function lcdSmartResponse(body: unknown) {
  return {
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({ data: body }),
  }
}

export function factoryAddressFromEnv(): string {
  let addr = process.env.VITE_FACTORY_ADDRESS?.trim() ?? ''
  if (!addr.startsWith('terra1')) {
    const envLocal = path.join(repoRootFromE2e(), 'frontend-dapp', '.env.local')
    const text = fs.readFileSync(envLocal, 'utf8')
    const m = text.match(/^VITE_FACTORY_ADDRESS=(.+)$/m)
    addr = m?.[1]?.trim().replace(/^["']|["']$/g, '') ?? ''
  }
  if (!addr.startsWith('terra1')) {
    throw new Error('VITE_FACTORY_ADDRESS missing — run make deploy-local (writes frontend-dapp/.env.local)')
  }
  return addr
}

/** Intercept factory `blacklist_check` for isolated trading-blacklist CTA tests (GitLab #388 / #422). */
export async function routeTradingBlacklistCheck(page: Page, factoryAddress: string, resp: BlacklistCheckLcdResponse) {
  await page.route('**/*', async (route: Route) => {
    const url = route.request().url()
    if (!url.includes('/cosmwasm/wasm/v1/contract/')) {
      await route.continue()
      return
    }
    const decoded = decodeSmartQuery(url)
    if (!decoded || decoded.contract !== factoryAddress) {
      await route.continue()
      return
    }
    if (decoded.query.blacklist_check !== undefined) {
      await route.fulfill(lcdSmartResponse(resp))
      return
    }
    await route.continue()
  })
}

export function walletBlacklistedLcdResponse(): BlacklistCheckLcdResponse {
  return {
    blocked: true,
    wallet_blacklisted: true,
    blacklisted_tokens: [],
    pair_blacklisted: false,
    blacklisted_pairs: [],
  }
}

/** Wallet blacklist alert copy (matches `describeTradingBlacklistBlock` / GitLab #388). */
export const WALLET_BLACKLIST_ALERT_COPY =
  'This wallet is on the protocol trading blacklist for compliance or incident response. ' +
  'Swaps, liquidity, and limit orders are disabled until governance removes the restriction.'
