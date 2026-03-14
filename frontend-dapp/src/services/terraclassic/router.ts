import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { ROUTER_CONTRACT_ADDRESS } from '@/utils/constants'
import type { AssetInfo, PairInfo } from '@/types'
import { tokenAssetInfo, assetInfoLabel } from '@/types'

export interface SwapOperation {
  terra_swap: {
    offer_asset_info: AssetInfo
    ask_asset_info: AssetInfo
  }
}

interface SimulateResponse {
  amount: string
}

export async function simulateMultiHopSwap(
  offerAmount: string,
  operations: SwapOperation[]
): Promise<SimulateResponse> {
  return queryContract<SimulateResponse>(ROUTER_CONTRACT_ADDRESS, {
    simulate_swap_operations: {
      offer_amount: offerAmount,
      operations: operations.map((op) => ({ terra_swap: op.terra_swap })),
    },
  })
}

export async function reverseSimulateMultiHopSwap(
  askAmount: string,
  operations: SwapOperation[]
): Promise<SimulateResponse> {
  return queryContract<SimulateResponse>(ROUTER_CONTRACT_ADDRESS, {
    reverse_simulate_swap_operations: {
      ask_amount: askAmount,
      operations: operations.map((op) => ({ terra_swap: op.terra_swap })),
    },
  })
}

export async function executeMultiHopSwap(
  walletAddress: string,
  inputTokenAddress: string,
  amount: string,
  operations: SwapOperation[],
  minimumReceive?: string,
  to?: string,
  deadline?: number
): Promise<string> {
  const swapMsg = btoa(
    JSON.stringify({
      execute_swap_operations: {
        operations: operations.map((op) => ({ terra_swap: op.terra_swap })),
        minimum_receive: minimumReceive,
        to,
        deadline,
      },
    })
  )
  return executeTerraContract(walletAddress, inputTokenAddress, {
    send: {
      contract: ROUTER_CONTRACT_ADDRESS,
      amount,
      msg: swapMsg,
    },
  })
}

/**
 * Build a graph of token connections from all pairs, then find the shortest
 * path (BFS) between two tokens. Returns the route as SwapOperation[].
 * Max 4 hops.
 */
export function findRoute(
  pairs: PairInfo[],
  fromToken: string,
  toToken: string
): SwapOperation[] | null {
  if (fromToken === toToken) return null

  const graph = new Map<string, { token: string; pair: PairInfo }[]>()

  for (const pair of pairs) {
    const tokenA = assetInfoLabel(pair.asset_infos[0])
    const tokenB = assetInfoLabel(pair.asset_infos[1])

    if (!graph.has(tokenA)) graph.set(tokenA, [])
    if (!graph.has(tokenB)) graph.set(tokenB, [])

    graph.get(tokenA)!.push({ token: tokenB, pair })
    graph.get(tokenB)!.push({ token: tokenA, pair })
  }

  const visited = new Set<string>()
  const queue: { token: string; path: SwapOperation[] }[] = [
    { token: fromToken, path: [] },
  ]
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

/**
 * Extract all unique tokens from the pairs list.
 */
export function getAllTokens(pairs: PairInfo[]): string[] {
  const tokens = new Set<string>()
  for (const pair of pairs) {
    tokens.add(assetInfoLabel(pair.asset_infos[0]))
    tokens.add(assetInfoLabel(pair.asset_infos[1]))
  }
  return Array.from(tokens)
}
