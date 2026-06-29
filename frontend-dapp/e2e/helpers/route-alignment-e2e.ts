import { expect, type APIRequestContext } from '@playwright/test'

import { lcdRequestGet } from './lcd-docker-fallback'
import type { TxWasmSwapHop } from './lcd'

const NATIVE_DENOM_SYMBOL: Record<string, string> = {
  uluna: 'LUNC',
  uusd: 'USTC',
}

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

/** Parse `TOKEN_A → TOKEN_B → …` from swap-route-summary mono line (SEC-E07). */
export function parseDisplayedRouteSymbols(routeLine: string): string[] {
  const cleaned = routeLine
    .replace(/Wrap \(1:1\)|Unwrap \(1:1\)/g, '')
    .replace(/\s+/g, ' ')
    .trim()
  if (!cleaned) return []
  return cleaned
    .split(/\s*→\s*/)
    .map((s) => s.trim())
    .filter(Boolean)
}

/** Inclusive token path from ordered wasm swap hops (offer of hop0, then each ask). */
export function tokenPathFromWasmSwapHops(hops: TxWasmSwapHop[]): string[] {
  if (hops.length === 0) return []
  const out = [hops[0].offerAsset]
  for (const hop of hops) out.push(hop.askAsset)
  return out
}

/** No duplicate or back-to-back identical segments in a route path. */
export function assertNoDuplicateRouteSegments(symbols: string[], label: string): void {
  for (let i = 1; i < symbols.length; i++) {
    expect(symbols[i], `${label}: consecutive duplicate at ${i}`).not.toBe(symbols[i - 1])
  }
  const seen = new Set<string>()
  for (const sym of symbols) {
    expect(seen.has(sym), `${label}: duplicate token ${sym}`).toBe(false)
    seen.add(sym)
  }
}

async function cw20Symbol(request: APIRequestContext, contract: string): Promise<string> {
  const q = b64SmartQuery({ token_info: {} })
  const res = await lcdRequestGet(request, `/cosmwasm/wasm/v1/contract/${contract}/smart/${q}`, {
    timeout: 20_000,
  })
  expect(res.ok, `token_info for ${contract}`).toBe(true)
  const body = (await res.json()) as { data?: { symbol?: string } | string }
  const decoded = decodeSmartDataPayload<{ symbol?: string }>(body)
  expect(decoded?.symbol, `symbol for ${contract}`).toBeTruthy()
  return decoded!.symbol!.toUpperCase()
}

/** Map on-chain offer/ask id (denom or CW20) to UI symbol. */
export async function resolveAssetSymbol(request: APIRequestContext, assetId: string): Promise<string> {
  const native = NATIVE_DENOM_SYMBOL[assetId.toLowerCase()]
  if (native) return native
  if (assetId.startsWith('terra1')) return cw20Symbol(request, assetId)
  return assetId.toUpperCase()
}

/** Resolve wasm hop path to uppercase symbols for comparison with UI route row. */
export async function wasmSwapHopsToSymbols(request: APIRequestContext, hops: TxWasmSwapHop[]): Promise<string[]> {
  const ids = tokenPathFromWasmSwapHops(hops)
  const symbols: string[] = []
  for (const id of ids) {
    symbols.push(await resolveAssetSymbol(request, id))
  }
  return symbols
}

/** Assert displayed route symbols match on-chain wasm swap hop sequence (SEC-E07). */
export async function assertDisplayedRouteMatchesTxHops(
  request: APIRequestContext,
  displayedSymbols: string[],
  hops: TxWasmSwapHop[],
  context: string
): Promise<void> {
  assertNoDuplicateRouteSegments(displayedSymbols, `${context} display`)
  const onChainSymbols = await wasmSwapHopsToSymbols(request, hops)
  assertNoDuplicateRouteSegments(onChainSymbols, `${context} tx`)
  expect(onChainSymbols, `${context}: hop count`).toHaveLength(displayedSymbols.length)
  expect(onChainSymbols, `${context}: symbol sequence`).toEqual(displayedSymbols.map((s) => s.toUpperCase()))
}
