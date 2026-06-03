import { execFileSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { expect, test, type APIRequestContext } from '@playwright/test'

import { lcdRequestGet } from './lcd-docker-fallback'

const DEV_WALLET = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const DEFAULT_INDEXER = 'http://127.0.0.1:3001'
const PARKED_POLL_TIMEOUT_MS = 180_000

export type IndexerLimitPlacementRow = {
  order_id: number
  owner: string
  lifecycle_status: string
}

function repoRootFromE2e(): string {
  const here = path.dirname(fileURLToPath(import.meta.url))
  return path.resolve(here, '..', '..', '..')
}

/** Run terrad seed script: place expired bids → wait → hybrid park (GitLab #259). */
export function seedExpiredParkedLimitsForClaimAllE2e(): void {
  const repoRoot = repoRootFromE2e()
  const script = path.join(repoRoot, 'scripts', 'e2e-seed-expired-parked-claim-all.sh')
  execFileSync('bash', [script], {
    stdio: 'inherit',
    env: { ...process.env, REPO_ROOT: repoRoot },
    cwd: repoRoot,
  })
}

export function indexerBaseUrl(): string {
  return (process.env.VITE_INDEXER_URL ?? DEFAULT_INDEXER).replace(/\/$/, '')
}

export async function fetchParkedExpiredPlacementsForDevWallet(
  request: APIRequestContext,
  pairAddr: string,
  minCount = 2
): Promise<IndexerLimitPlacementRow[]> {
  const base = indexerBaseUrl()
  const url = `${base}/api/v1/pairs/${pairAddr}/limit-placements?status=parked_expired`
  let last: IndexerLimitPlacementRow[] = []
  await expect(async () => {
    const res = await request.get(url, { timeout: 20_000, failOnStatusCode: false })
    if (!res.ok()) {
      throw new Error(`indexer limit-placements ${res.status()} for ${pairAddr}`)
    }
    const body = (await res.json()) as IndexerLimitPlacementRow[]
    last = body.filter((r) => r.owner === DEV_WALLET && r.lifecycle_status === 'parked_expired')
    if (last.length < minCount) {
      throw new Error(`expected >= ${minCount} parked_expired rows for dev wallet, got ${last.length}`)
    }
  }).toPass({ timeout: PARKED_POLL_TIMEOUT_MS })
  return last
}

/** First dual-CW20 unpaused pair address from factory LCD list (same order as limit E2E). */
export async function firstDualCwPairAddr(request: APIRequestContext): Promise<string> {
  const envLocal = path.join(repoRootFromE2e(), 'frontend-dapp', '.env.local')
  let factory = process.env.VITE_FACTORY_ADDRESS ?? ''
  if (!factory) {
    const fs = await import('node:fs')
    const text = fs.readFileSync(envLocal, 'utf8')
    const m = text.match(/^VITE_FACTORY_ADDRESS=(.+)$/m)
    factory = m?.[1]?.trim().replace(/^["']|["']$/g, '') ?? ''
  }
  if (!factory.startsWith('terra1')) {
    test.fail(true, 'VITE_FACTORY_ADDRESS missing — run scripts/deploy-dex-local.sh')
    return ''
  }

  const q = Buffer.from(JSON.stringify({ pairs: { start_after: null, limit: 60 } })).toString('base64')
  const res = await lcdRequestGet(request, `/cosmwasm/wasm/v1/contract/${factory}/smart/${q}`, {
    timeout: 20_000,
  })
  expect(res.ok).toBeTruthy()
  const raw = (await res.json()) as { data?: unknown }
  let pairsDoc: { pairs?: Array<{ contract_addr: string; asset_infos: unknown[] }> }
  if (typeof raw.data === 'string') {
    pairsDoc = JSON.parse(Buffer.from(raw.data, 'base64').toString('utf8')) as typeof pairsDoc
  } else {
    pairsDoc = raw.data as typeof pairsDoc
  }

  for (const p of pairsDoc.pairs ?? []) {
    const infos = p.asset_infos as Array<{ token?: { contract_addr?: string } }>
    const a = infos[0]?.token?.contract_addr ?? ''
    const b = infos[1]?.token?.contract_addr ?? ''
    if (a.startsWith('terra1') && b.startsWith('terra1')) {
      const pausedQ = Buffer.from(JSON.stringify({ is_paused: {} })).toString('base64')
      const pausedRes = await lcdRequestGet(request, `/cosmwasm/wasm/v1/contract/${p.contract_addr}/smart/${pausedQ}`)
      if (!pausedRes.ok) continue
      const pausedRaw = (await pausedRes.json()) as { data?: unknown }
      let paused = false
      if (typeof pausedRaw.data === 'string') {
        paused = JSON.parse(Buffer.from(pausedRaw.data, 'base64').toString('utf8')).paused === true
      } else {
        paused = (pausedRaw.data as { paused?: boolean })?.paused === true
      }
      if (!paused) return p.contract_addr
    }
  }
  test.fail(true, 'No unpaused dual-CW20 pair for expired-park E2E')
  return ''
}

export { DEV_WALLET }
