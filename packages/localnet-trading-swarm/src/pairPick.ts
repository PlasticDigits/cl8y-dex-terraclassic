import type { PairInfo } from './types.js'
import { assetInfoLabel } from './types.js'
import { findRoute } from './graph.js'
import type { SwapOperation } from './types.js'

export function pairTouchesTax(pair: PairInfo, taxTokens: Set<string>): boolean {
  return pair.asset_infos.some((ai) => taxTokens.has(assetInfoLabel(ai)))
}

/** Gem / OE-1 workers: drop any pair that includes a tax token. */
export function filterGemPairs(pairs: PairInfo[], taxTokens: Set<string>): PairInfo[] {
  if (taxTokens.size === 0) return pairs
  return pairs.filter((p) => !pairTouchesTax(p, taxTokens))
}

/** Tax workers: only the listed tax/EMBER (or other tax) pair. */
export function filterTaxPairs(pairs: PairInfo[], taxTokens: Set<string>): PairInfo[] {
  if (taxTokens.size === 0) return []
  return pairs.filter((p) => pairTouchesTax(p, taxTokens))
}

export function cw20AddrsFromPairs(pairs: PairInfo[], exclude?: Set<string>): string[] {
  const tokens = new Set<string>()
  for (const p of pairs) {
    for (const ai of p.asset_infos) {
      const x = assetInfoLabel(ai)
      if (!x.startsWith('terra1')) continue
      if (exclude?.has(x)) continue
      tokens.add(x)
    }
  }
  return [...tokens]
}

export function randomCw20PairEndpoints(
  pairs: PairInfo[],
  exclude?: Set<string>
): { from: string; to: string } | null {
  const arr = cw20AddrsFromPairs(pairs, exclude)
  if (arr.length < 2) return null
  const from = arr[Math.floor(Math.random() * arr.length)]!
  let to = arr[Math.floor(Math.random() * arr.length)]!
  let guard = 0
  while (to === from && guard++ < 8) {
    to = arr[Math.floor(Math.random() * arr.length)]!
  }
  if (to === from) return null
  return { from, to }
}

/**
 * Official-router ≥2hop that includes a tax token (sell tax first, else buy last).
 * Graph uses **all** factory pairs so TAX→EMBER→CORAL is reachable.
 */
export function findTaxInclusiveRoute(
  allPairs: PairInfo[],
  taxTokens: Set<string>,
  preferSell = true
): { from: string; to: string; route: SwapOperation[] } | null {
  if (taxTokens.size === 0) return null
  const tokens = cw20AddrsFromPairs(allPairs)
  const taxList = [...taxTokens].filter((t) => tokens.includes(t))
  const others = tokens.filter((t) => !taxTokens.has(t))
  if (taxList.length === 0 || others.length === 0) return null

  const starts = preferSell ? taxList : others
  const ends = preferSell ? others : taxList
  const shuffledStarts = [...starts].sort(() => Math.random() - 0.5)
  const shuffledEnds = [...ends].sort(() => Math.random() - 0.5)

  for (const from of shuffledStarts) {
    for (const to of shuffledEnds) {
      const route = findRoute(allPairs, from, to)
      if (route && route.length >= 2) {
        return { from, to, route }
      }
    }
  }
  return null
}
