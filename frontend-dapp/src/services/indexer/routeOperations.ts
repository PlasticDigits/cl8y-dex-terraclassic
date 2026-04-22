import type { AssetInfo, HybridSwapParams } from '@/types'
import { tokenAssetInfo } from '@/types'
import type { SwapOperation } from '@/services/terraclassic/router'

function assetInfoFromIndexerJson(v: unknown): AssetInfo {
  if (!v || typeof v !== 'object') throw new Error('invalid offer_asset_info / ask_asset_info')
  const o = v as Record<string, unknown>
  if (o.token && typeof o.token === 'object') {
    const addr = (o.token as Record<string, unknown>).contract_addr
    if (typeof addr === 'string' && addr.length > 0) return tokenAssetInfo(addr)
  }
  if (o.native_token && typeof o.native_token === 'object') {
    const denom = (o.native_token as Record<string, unknown>).denom
    if (typeof denom === 'string' && denom.length > 0) {
      return { native_token: { denom } }
    }
  }
  throw new Error('unsupported asset info in indexer router_operations')
}

function hybridFromJson(h: unknown): HybridSwapParams | undefined {
  if (h == null) return undefined
  if (typeof h !== 'object') return undefined
  const o = h as Record<string, unknown>
  const pool = String(o.pool_input ?? '0')
  const book = String(o.book_input ?? '0')
  if (pool === '0' && book === '0') return undefined
  return {
    pool_input: pool,
    book_input: book,
    max_maker_fills: Number(o.max_maker_fills ?? 8),
    book_start_hint:
      o.book_start_hint === null || o.book_start_hint === undefined
        ? null
        : Number(o.book_start_hint),
  }
}

/** Map indexer `router_operations` to wallet/router `SwapOperation[]` (must match `hops.length`). */
export function swapOperationsFromIndexerResponse(
  routerOperations: unknown[],
  hopCount: number
): SwapOperation[] {
  if (!Array.isArray(routerOperations) || routerOperations.length !== hopCount) {
    throw new Error('router_operations must be an array with one entry per hop')
  }
  const out: SwapOperation[] = []
  for (const op of routerOperations) {
    if (!op || typeof op !== 'object') throw new Error('invalid router operation')
    const ts = (op as Record<string, unknown>).terra_swap as Record<string, unknown> | undefined
    if (!ts) throw new Error('expected terra_swap in router operation')
    const offer = assetInfoFromIndexerJson(ts.offer_asset_info)
    const ask = assetInfoFromIndexerJson(ts.ask_asset_info)
    const hybrid = hybridFromJson(ts.hybrid)
    out.push({
      terra_swap: {
        offer_asset_info: offer,
        ask_asset_info: ask,
        ...(hybrid ? { hybrid } : {}),
      },
    })
  }
  return out
}
