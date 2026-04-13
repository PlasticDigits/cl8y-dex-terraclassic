import { expect, type APIRequestContext, type Page } from '@playwright/test'

import { lcdBaseUrl } from './chain'

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

/** Whether LCD tx JSON includes a wasm event whose last `action` attribute equals `action`. */
export function txJsonHasWasmAction(txJson: unknown, action: string): boolean {
  const root = txJson as Record<string, unknown>
  const tr = (root.tx_response as Record<string, unknown> | undefined) ?? root
  for (const ev of collectTxEvents(tr)) {
    if (ev.type !== 'wasm') continue
    if (wasmAttrLast(ev.attributes ?? [], 'action') === action) return true
  }
  return false
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
