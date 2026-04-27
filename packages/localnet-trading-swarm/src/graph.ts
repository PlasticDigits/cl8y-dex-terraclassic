import type { PairInfo, SwapOperation } from './types.js'
import { assetInfoLabel, tokenAssetInfo } from './types.js'

/** BFS route between CW20 tokens (max 4 hops). Mirrors frontend `findRoute`. */
export function findRoute(pairs: PairInfo[], fromToken: string, toToken: string): SwapOperation[] | null {
  if (fromToken === toToken) return null

  const graph = new Map<string, { token: string; pair: PairInfo }[]>()

  for (const pair of pairs) {
    const tokenA = assetInfoLabel(pair.asset_infos[0])
    const tokenB = assetInfoLabel(pair.asset_infos[1])
    if (!tokenA.startsWith('terra1') || !tokenB.startsWith('terra1')) continue

    if (!graph.has(tokenA)) graph.set(tokenA, [])
    if (!graph.has(tokenB)) graph.set(tokenB, [])
    graph.get(tokenA)!.push({ token: tokenB, pair })
    graph.get(tokenB)!.push({ token: tokenA, pair })
  }

  const visited = new Set<string>()
  const queue: { token: string; path: SwapOperation[] }[] = [{ token: fromToken, path: [] }]
  visited.add(fromToken)

  while (queue.length > 0) {
    const current = queue.shift()!
    if (current.path.length >= 4) continue

    const neighbors = graph.get(current.token) ?? []
    for (const { token: nextToken } of neighbors) {
      if (visited.has(nextToken)) continue
      visited.add(nextToken)

      const op: SwapOperation = {
        terra_swap: {
          offer_asset_info: tokenAssetInfo(current.token),
          ask_asset_info: tokenAssetInfo(nextToken),
        },
      }
      const newPath = [...current.path, op]

      if (nextToken === toToken) return newPath

      queue.push({ token: nextToken, path: newPath })
    }
  }

  return null
}
