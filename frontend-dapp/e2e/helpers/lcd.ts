import { expect, type APIRequestContext, type Page } from '@playwright/test'

import { lcdBaseUrl } from './chain'

/** Pair `is_paused` smart-query response (`data` may be JSON or base64 string). */
export type LcdPairPausedResponse = { paused: boolean }

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

function b64SmartQuery(msg: Record<string, unknown>): string {
  return Buffer.from(JSON.stringify(msg)).toString('base64')
}

/** LCD `is_paused` on a pair contract. Returns `true` when the query fails (conservative for strict E2E). */
export async function queryPairPaused(request: APIRequestContext, pairAddr: string): Promise<boolean> {
  const base = lcdBaseUrl()
  const q = b64SmartQuery({ is_paused: {} })
  const res = await request.get(`${base}/cosmwasm/wasm/v1/contract/${pairAddr}/smart/${q}`, {
    failOnStatusCode: false,
    timeout: 20_000,
  })
  if (!res.ok()) return true
  const body = (await res.json()) as { data?: LcdPairPausedResponse | string }
  const decoded = decodeSmartDataPayload<LcdPairPausedResponse>(body)
  return decoded?.paused === true
}

/** First page of factory `pairs` query (same shape as LCD `smart` JSON `data`). */
export type LcdPairAssetInfo =
  | { token: { contract_addr: string }; native_token?: never }
  | { native_token: { denom: string }; token?: never }

export type LcdPairInfo = {
  contract_addr: string
  asset_infos: [LcdPairAssetInfo, LcdPairAssetInfo]
}

export function assetInfoLabel(info: LcdPairAssetInfo): string {
  if ('token' in info && info.token) return info.token.contract_addr
  return info.native_token.denom
}

export function firstDualCwPair(pairs: LcdPairInfo[]): { pair: LcdPairInfo; index: number } | null {
  for (let i = 0; i < pairs.length; i++) {
    const p = pairs[i]
    const a = assetInfoLabel(p.asset_infos[0])
    const b = assetInfoLabel(p.asset_infos[1])
    if (a.startsWith('terra1') && b.startsWith('terra1')) return { pair: p, index: i }
  }
  return null
}

/** First dual-CW20 factory pair whose `is_paused` query is false (factory list order). */
export async function firstUnpausedDualCwPair(
  request: APIRequestContext,
  pairs: LcdPairInfo[]
): Promise<{ pair: LcdPairInfo; index: number } | null> {
  for (let i = 0; i < pairs.length; i++) {
    const p = pairs[i]
    const a = assetInfoLabel(p.asset_infos[0])
    const b = assetInfoLabel(p.asset_infos[1])
    if (!a.startsWith('terra1') || !b.startsWith('terra1')) continue
    if (!(await queryPairPaused(request, p.contract_addr))) return { pair: p, index: i }
  }
  return null
}

function isFactoryPairsSmartUrl(url: string): boolean {
  if (!url.includes('/cosmwasm/wasm/v1/contract/') || !url.includes('/smart/')) return false
  const seg = decodeURIComponent(url.split('/smart/')[1]?.split(/[?#]/)[0] ?? '')
  try {
    const decoded = Buffer.from(seg, 'base64').toString('utf8')
    const q = JSON.parse(decoded) as Record<string, unknown>
    return 'pairs' in q
  } catch {
    return false
  }
}

/**
 * Navigates and returns `data.pairs` from the first factory `pairs` LCD response on that load.
 */
export async function gotoAndCaptureFactoryPairsPage(page: Page, path: string): Promise<LcdPairInfo[]> {
  const [resp] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === 'GET' && r.status() === 200 && isFactoryPairsSmartUrl(r.url()),
      { timeout: 90_000 }
    ),
    page.goto(path),
  ])
  const json = (await resp.json()) as { data?: { pairs?: LcdPairInfo[] } }
  return json.data?.pairs ?? []
}

/** Use after navigation (e.g. dev wallet already opened `/`) to read the first factory `pairs` page from LCD. */
export async function reloadAndCaptureFactoryPairsPage(page: Page): Promise<LcdPairInfo[]> {
  const [resp] = await Promise.all([
    page.waitForResponse(
      (r) => r.request().method() === 'GET' && r.status() === 200 && isFactoryPairsSmartUrl(r.url()),
      { timeout: 90_000 }
    ),
    page.reload({ waitUntil: 'networkidle' }),
  ])
  const json = (await resp.json()) as { data?: { pairs?: LcdPairInfo[] } }
  return json.data?.pairs ?? []
}

function wasmAttrLast(attrs: Array<{ key: string; value: string }>, key: string): string | undefined {
  for (let i = attrs.length - 1; i >= 0; i--) {
    if (attrs[i].key === key) return attrs[i].value
  }
  return undefined
}

function wasmHasAttrValue(attrs: Array<{ key: string; value: string }>, key: string, value: string): boolean {
  return attrs.some((a) => a.key === key && a.value === value)
}

function collectTxEvents(
  txResponse: Record<string, unknown>
): Array<{ type: string; attributes: Array<{ key: string; value: string }> }> {
  const logs = txResponse.logs
  if (Array.isArray(logs)) {
    return logs.flatMap((l: { events?: unknown }) => (Array.isArray(l.events) ? l.events : []))
  }
  const ev = txResponse.events
  return Array.isArray(ev) ? ev : []
}

function isWasmLikeEventType(type: string): boolean {
  // Terra LCD emits submessage callbacks as `wasm-wasm`; top-level contract calls use `wasm`.
  return type === 'wasm' || type === 'wasm-wasm'
}

/** Whether LCD tx JSON includes a wasm / wasm-wasm event with an `action` attribute equal to `action`. */
export function txJsonHasWasmAction(txJson: unknown, action: string): boolean {
  const root = txJson as Record<string, unknown>
  const tr = (root.tx_response as Record<string, unknown> | undefined) ?? root
  for (const ev of collectTxEvents(tr)) {
    if (!isWasmLikeEventType(ev.type)) continue
    if (wasmHasAttrValue(ev.attributes ?? [], 'action', action)) return true
  }
  return false
}

/** Last wasm attribute value for `key` on events whose last `action` equals `action`. */
export function txJsonWasmAttrForAction(txJson: unknown, action: string, key: string): string | undefined {
  const root = txJson as Record<string, unknown>
  const tr = (root.tx_response as Record<string, unknown> | undefined) ?? root
  for (const ev of collectTxEvents(tr)) {
    if (!isWasmLikeEventType(ev.type)) continue
    if (wasmAttrLast(ev.attributes ?? [], 'action') !== action) continue
    return wasmAttrLast(ev.attributes ?? [], key)
  }
  return undefined
}

/** Last `return_amount` on wasm `swap` events (multihop router: final hop output). */
export function txJsonLastSwapReturnAmount(txJson: unknown): string | undefined {
  const amounts = txJsonAllSwapReturnAmounts(txJson)
  return amounts.length > 0 ? amounts[amounts.length - 1] : undefined
}

/** Every `return_amount` on wasm `swap` events in tx order (multihop router). */
export function txJsonAllSwapReturnAmounts(txJson: unknown): string[] {
  const root = txJson as Record<string, unknown>
  const tr = (root.tx_response as Record<string, unknown> | undefined) ?? root
  const out: string[] = []
  for (const ev of collectTxEvents(tr)) {
    if (!isWasmLikeEventType(ev.type)) continue
    if (wasmAttrLast(ev.attributes ?? [], 'action') !== 'swap') continue
    const ret = wasmAttrLast(ev.attributes ?? [], 'return_amount')
    if (ret) out.push(ret)
  }
  return out
}

type LcdExecuteMsg = Record<string, unknown>

/** Decode `max_adjust_steps` from the CW20 `send` hook in a place-limit batch tx (GitLab #204). */
export function txJsonPlaceLimitMaxAdjustSteps(txJson: unknown): number | null {
  const root = txJson as Record<string, unknown>
  const tx = root.tx as { body?: { messages?: Array<Record<string, unknown>> } } | undefined
  const messages = tx?.body?.messages ?? []
  for (const m of messages) {
    const type = String(m['@type'] ?? '')
    if (!type.includes('MsgExecuteContract')) continue
    const msg = m.msg as LcdExecuteMsg | string | undefined
    if (!msg || typeof msg === 'string') continue
    const send = msg.send as { msg?: string } | undefined
    if (!send?.msg || typeof send.msg !== 'string') continue
    try {
      const hook = JSON.parse(Buffer.from(send.msg, 'base64').toString('utf8')) as {
        place_limit_order_batch?: { orders?: Array<{ max_adjust_steps?: number }> }
      }
      const steps = hook.place_limit_order_batch?.orders?.[0]?.max_adjust_steps
      if (typeof steps === 'number' && Number.isFinite(steps)) return steps
    } catch {
      /* try next execute msg */
    }
  }
  return null
}

export async function fetchTxJson(request: APIRequestContext, txHash: string): Promise<unknown | null> {
  const base = lcdBaseUrl()
  const candidates = [txHash, txHash.toUpperCase(), txHash.toLowerCase()]
  const uniq = [...new Set(candidates)]
  for (const h of uniq) {
    const res = await request.get(`${base}/cosmos/tx/v1beta1/txs/${encodeURIComponent(h)}`, {
      failOnStatusCode: false,
      timeout: 20_000,
    })
    if (res.ok()) return res.json()
  }
  return null
}

export async function readTxHashFromAlertLink(page: Page, alert: ReturnType<Page['locator']>): Promise<string> {
  const link = alert.locator('a[title]')
  await expect(link).toBeVisible({ timeout: 90_000 })
  const h = await link.getAttribute('title')
  expect(h, 'success alert should include full tx hash in link title').toBeTruthy()
  return h as string
}
