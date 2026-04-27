import type { AssetInfo, PairInfo } from './types.js'
import { assetInfoLabel } from './types.js'
import { queryWasmSmart } from './lcd.js'

const PAGE = 60

/** Paginated factory `pairs` query; returns all `PairInfo` rows. */
export async function fetchAllPairs(lcdBase: string, factory: string): Promise<PairInfo[]> {
  const all: PairInfo[] = []
  let start_after: [AssetInfo, AssetInfo] | null = null

  for (;;) {
    const res: { pairs: PairInfo[] } = await queryWasmSmart(lcdBase, factory, {
      pairs: { start_after, limit: PAGE },
    })
    const page: PairInfo[] = res.pairs ?? []
    if (page.length === 0) break
    all.push(...page)
    const last: PairInfo = page[page.length - 1]!
    start_after = [last.asset_infos[0], last.asset_infos[1]]
    if (page.length < PAGE) break
  }
  return all
}

/** Unique CW20 contract addresses appearing in any factory pair (full enumeration for mint funding). */
export function uniqueCw20TokenAddresses(pairs: PairInfo[]): string[] {
  const set = new Set<string>()
  for (const p of pairs) {
    for (const info of p.asset_infos) {
      if ('token' in info) {
        const a = info.token.contract_addr
        if (a) set.add(a)
      }
    }
  }
  return [...set].sort()
}

/** CW20-only tokens for routing (graph edges). */
export function cw20TokensFromPairs(pairs: PairInfo[]): string[] {
  const set = new Set<string>()
  for (const p of pairs) {
    for (const info of p.asset_infos) {
      const label = assetInfoLabel(info)
      if (label.startsWith('terra1')) set.add(label)
    }
  }
  return [...set]
}
