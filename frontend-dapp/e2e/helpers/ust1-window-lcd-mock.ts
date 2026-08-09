import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import type { Page, Route } from '@playwright/test'

/** Published columbus-5 ust1-window (GitLab #506) — used when local env lacks window address. */
export const MAINNET_UST1_WINDOW = 'terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2'
export const MAINNET_UST1_TOKEN = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
export const MAINNET_VFDUSD_TOKEN = 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3'

const RATE_1E18 = '1000000000000000000'

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

export type Ust1EffectiveSwapMock = {
  fee_bps?: number
  per_tx_ust1_limit?: string
  rolling_24h_ust1_limit?: string
  paused?: boolean
  rolling_window_start_sec?: number
  rolling_volume_ust1?: string
  max_oracle_age_sec?: number
  oracle?: {
    rate?: string
    last_update_sec?: number
    paused?: boolean
  }
}

export function healthyUst1EffectiveSwap(overrides: Ust1EffectiveSwapMock = {}) {
  const now = Math.floor(Date.now() / 1000)
  return {
    fee_bps: overrides.fee_bps ?? 100,
    per_tx_ust1_limit: overrides.per_tx_ust1_limit ?? '1000000000',
    rolling_24h_ust1_limit: overrides.rolling_24h_ust1_limit ?? '10000000000',
    paused: overrides.paused ?? false,
    rolling_window_start_sec: overrides.rolling_window_start_sec ?? now - 100,
    rolling_volume_ust1: overrides.rolling_volume_ust1 ?? '0',
    max_oracle_age_sec: overrides.max_oracle_age_sec ?? 21_600,
    oracle: {
      rate: overrides.oracle?.rate ?? RATE_1E18,
      last_update_sec: overrides.oracle?.last_update_sec ?? now - 30,
      paused: overrides.oracle?.paused ?? false,
    },
  }
}

/**
 * Intercept ust1-window LCD `effective_swap` + CW20 balances for UST1/vFDUSD
 * so CTA enable tests are not blocked by missing LocalTerra token balances (#506).
 */
export async function routeUst1EffectiveSwap(
  page: Page,
  windowAddress: string,
  body: Ust1EffectiveSwapMock = {},
  opts: { balanceRaw?: string; tokenAddresses?: string[] } = {}
) {
  const payload = healthyUst1EffectiveSwap(body)
  const balanceRaw = opts.balanceRaw ?? '100000000'
  const tokenAddresses = new Set(
    (opts.tokenAddresses ?? [MAINNET_UST1_TOKEN, MAINNET_VFDUSD_TOKEN]).map((a) => a.toLowerCase())
  )
  await page.route('**/*', async (route: Route) => {
    const url = route.request().url()
    if (!url.includes('/cosmwasm/wasm/v1/contract/')) {
      await route.continue()
      return
    }
    const decoded = decodeSmartQuery(url)
    if (!decoded) {
      await route.continue()
      return
    }
    if (decoded.contract === windowAddress && decoded.query.effective_swap !== undefined) {
      await route.fulfill(lcdSmartResponse(payload))
      return
    }
    if (tokenAddresses.has(decoded.contract.toLowerCase()) && decoded.query.balance !== undefined) {
      await route.fulfill(lcdSmartResponse({ balance: balanceRaw }))
      return
    }
    await route.continue()
  })
}

function readEnvLocalKey(key: string): string {
  const envLocal = path.join(repoRootFromE2e(), 'frontend-dapp', '.env.local')
  if (!fs.existsSync(envLocal)) return ''
  const text = fs.readFileSync(envLocal, 'utf8')
  const m = text.match(new RegExp(`^${key}=(.+)$`, 'm'))
  return m?.[1]?.trim().replace(/^["']|["']$/g, '') ?? ''
}

export function ust1WindowAddressFromEnv(): string {
  let addr = process.env.VITE_UST1_WINDOW_ADDRESS?.trim() ?? ''
  if (!addr.startsWith('terra1')) addr = readEnvLocalKey('VITE_UST1_WINDOW_ADDRESS')
  if (!addr.startsWith('terra1')) addr = MAINNET_UST1_WINDOW
  return addr
}

/** Env values Playwright webServer should bake so `/ust1` is enabled without LocalTerra deploy. */
export const UST1_E2E_VITE_ENV = {
  VITE_UST1_WINDOW_ADDRESS: process.env.VITE_UST1_WINDOW_ADDRESS?.trim() || MAINNET_UST1_WINDOW,
  VITE_UST1_TOKEN_ADDRESS: process.env.VITE_UST1_TOKEN_ADDRESS?.trim() || MAINNET_UST1_TOKEN,
  VITE_VFDUSD_TOKEN_ADDRESS: process.env.VITE_VFDUSD_TOKEN_ADDRESS?.trim() || MAINNET_VFDUSD_TOKEN,
} as const
