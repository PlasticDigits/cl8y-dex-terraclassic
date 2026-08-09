import { expect, type APIRequestContext, type Page } from '@playwright/test'

import type { IndexerRouteSolveResponse } from '../../src/types'

import { captureRouterSimulateQuote } from './fee-discount-quote-e2e'
import type { LcdPairInfo } from './lcd'
import { assetInfoLabel } from './lcd'
import { lcdRequestGet } from './lcd-docker-fallback'

function b64SmartQuery(msg: Record<string, unknown>): string {
  return Buffer.from(JSON.stringify(msg)).toString('base64')
}

function decodeSmartDataPayload<T>(raw: { data?: T | string }): T | null {
  const data = raw.data
  if (data == null) return null
  if (typeof data === 'string') {
    try {
      return JSON.parse(Buffer.from(data, 'base64').toString('utf8')) as T
    } catch {
      return null
    }
  }
  return data as T
}

/** Pay CORAL → receive IRON: 2-hop path through seeded EMBER/CORAL ask book (hop 1 pays CORAL). */
export const MULTIHOP_HYBRID_PAY_SYMBOL = 'CORAL'
/** EMBER/IRON pool is liquid on LocalTerra seed; EMBER/COBALT is too thin for hop-2 spread checks. */
export const MULTIHOP_HYBRID_RECEIVE_SYMBOL = 'IRON'

const PREFERRED_RECEIVE_SYMBOLS = [MULTIHOP_HYBRID_RECEIVE_SYMBOL, 'cLUNC', 'COBALT'] as const

const MULTIHOP_ROUTE_MSG =
  'Indexer route/solve did not return ≥2 hops for multihop hybrid; verify deploy pairs and indexer (GitLab #422).'

const HYBRID_ROUTE_MSG =
  'Indexer route/solve did not attach hybrid on any hop; verify ask book seed (scripts/e2e-seed-hybrid-book.sh) and indexer (GitLab #422).'

const INDEXER_URL = process.env.VITE_INDEXER_URL ?? 'http://127.0.0.1:3001'

/** Raw pay amount that triggers hybrid book leg on CORAL→COBALT multihop (see route/solve grid). */
export const MULTIHOP_HYBRID_PAY_RAW = '600000000'

function routeHasHybridLeg(json: IndexerRouteSolveResponse): boolean {
  const ops = json.router_operations ?? []
  return ops.some((op) => {
    const swap = (op as { terra_swap?: { hybrid?: unknown } }).terra_swap
    const hybrid = swap?.hybrid
    return hybrid != null && typeof hybrid === 'object'
  })
}

async function tokenSymbol(request: APIRequestContext, contract: string): Promise<string | null> {
  const q = b64SmartQuery({ token_info: {} })
  const res = await lcdRequestGet(request, `/cosmwasm/wasm/v1/contract/${contract}/smart/${q}`, {
    timeout: 20_000,
  })
  if (!res.ok) return null
  const body = (await res.json()) as { data?: { symbol?: string } | string }
  const decoded = decodeSmartDataPayload<{ symbol?: string }>(body)
  return decoded?.symbol ?? null
}

async function multihopHybridReceiveAtAmount(
  request: APIRequestContext,
  payToken: string,
  outToken: string,
  amountRaw: string
): Promise<boolean> {
  const res = await request.get(
    `${INDEXER_URL}/api/v1/route/solve?token_in=${payToken}&token_out=${outToken}&amount_in=${amountRaw}`
  )
  if (!res.ok()) return false
  const json = (await res.json()) as IndexerRouteSolveResponse
  return (json.hops?.length ?? 0) >= 2 && routeHasHybridLeg(json)
}

/** Resolve CW20 contract for `symbol` from factory pair token addresses. */
export async function resolveTokenContractBySymbol(
  request: APIRequestContext,
  pairs: LcdPairInfo[],
  symbol: string
): Promise<string> {
  const candidates = [
    ...new Set(pairs.flatMap((p) => p.asset_infos.map(assetInfoLabel)).filter((t) => t.startsWith('terra1'))),
  ]
  for (const addr of candidates) {
    const sym = await tokenSymbol(request, addr)
    if (sym?.toUpperCase() === symbol.toUpperCase()) return addr
  }
  expect(false, `CW20 token ${symbol} not found on factory pairs`).toBe(true)
  return ''
}

/** Pick receive token with ≥2-hop CORAL→* route and hybrid on hop 0 at MULTIHOP_HYBRID_PAY_RAW. */
export async function resolveMultihopHybridReceiveToken(
  request: APIRequestContext,
  pairs: LcdPairInfo[],
  payToken: string
): Promise<string> {
  const preferred = await resolveTokenContractBySymbol(request, pairs, MULTIHOP_HYBRID_RECEIVE_SYMBOL)
  if (await multihopHybridReceiveAtAmount(request, payToken, preferred, MULTIHOP_HYBRID_PAY_RAW)) {
    return preferred
  }

  for (const symbol of PREFERRED_RECEIVE_SYMBOLS) {
    if (symbol === MULTIHOP_HYBRID_RECEIVE_SYMBOL) continue
    const addr = await resolveTokenContractBySymbol(request, pairs, symbol)
    if (await multihopHybridReceiveAtAmount(request, payToken, addr, MULTIHOP_HYBRID_PAY_RAW)) return addr
  }

  const candidates = [
    ...new Set(
      pairs.flatMap((p) => p.asset_infos.map(assetInfoLabel)).filter((t) => t.startsWith('terra1') && t !== payToken)
    ),
  ]
  for (const out of candidates) {
    if (out === preferred) continue
    if (await multihopHybridReceiveAtAmount(request, payToken, out, MULTIHOP_HYBRID_PAY_RAW)) return out
  }
  expect(
    false,
    `no multihop hybrid receive token for CORAL pay at ${MULTIHOP_HYBRID_PAY_RAW}; re-run e2e-seed-hybrid-book.sh`
  ).toBe(true)
  return ''
}

/** Waits for GET `/api/v1/route/solve` after token/amount selection. */
export async function captureIndexerRouteSolve(
  page: Page,
  opts: { minHops?: number; requireHybridLeg?: boolean; timeoutMs?: number } = {}
): Promise<IndexerRouteSolveResponse> {
  const minHops = opts.minHops ?? 2
  const timeout = opts.timeoutMs ?? 120_000
  const resp = await page.waitForResponse(
    (r) => r.request().method() === 'GET' && r.ok() && r.url().includes('/api/v1/route/solve'),
    { timeout }
  )
  const json = (await resp.json()) as IndexerRouteSolveResponse
  expect(json.hops?.length ?? 0, MULTIHOP_ROUTE_MSG).toBeGreaterThanOrEqual(minHops)
  if (opts.requireHybridLeg !== false) {
    expect(routeHasHybridLeg(json), HYBRID_ROUTE_MSG).toBe(true)
  }
  return json
}

export type MultihopHybridQuoteCapture = {
  routeSolve: IndexerRouteSolveResponse
  simulateReturnAmount: string
}

/** Fill pay amount and capture indexer multihop + router simulate quotes. */
export async function captureMultihopHybridQuote(
  page: Page,
  payAmountHuman: string,
  opts: { requireTrader?: string } = {}
): Promise<MultihopHybridQuoteCapture> {
  const routeSolvePromise = captureIndexerRouteSolve(page, { minHops: 2, requireHybridLeg: true })
  const simulatePromise = captureRouterSimulateQuote(page, opts)
  await page.getByPlaceholder('0.00').first().fill(payAmountHuman)
  const [routeSolve, simulate] = await Promise.all([routeSolvePromise, simulatePromise])
  return { routeSolve, simulateReturnAmount: simulate.returnAmount }
}

/** Executed raw return must be within `slippagePercent` of quoted simulate (symmetric floor). */
export function assertReturnWithinQuoteTolerance(
  quotedRaw: string,
  executedRaw: string,
  slippagePercent: number
): void {
  const quoted = BigInt(quotedRaw)
  const executed = BigInt(executedRaw)
  const floor = (quoted * BigInt(100 - slippagePercent)) / 100n
  expect(
    executed,
    `executed return ${executedRaw} below quote ${quotedRaw} minus ${slippagePercent}% slippage`
  ).toBeGreaterThanOrEqual(floor)
}
