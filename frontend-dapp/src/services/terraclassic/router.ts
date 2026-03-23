import { queryContract } from './queries'
import { executeTerraContract, executeTerraContractMulti } from './transactions'
import {
  ROUTER_CONTRACT_ADDRESS,
  NATIVE_WRAPPED_PAIRS,
  TREASURY_CONTRACT_ADDRESS,
  WRAP_MAPPER_CONTRACT_ADDRESS,
} from '@/utils/constants'
import type { AssetInfo, PairInfo } from '@/types'
import { tokenAssetInfo, assetInfoLabel, isNativeDenom, getWrappedEquivalent } from '@/types'

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
  maxSpread: string,
  minimumReceive?: string,
  to?: string,
  deadline?: number
): Promise<string> {
  const swapMsg = btoa(
    JSON.stringify({
      execute_swap_operations: {
        operations: operations.map((op) => ({ terra_swap: op.terra_swap })),
        max_spread: maxSpread,
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
export function findRoute(pairs: PairInfo[], fromToken: string, toToken: string): SwapOperation[] | null {
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

/**
 * Extract all unique tokens from the pairs list, including native LUNC/USTC
 * when their wrapped equivalents exist in the pair graph.
 */
export function getAllTokens(pairs: PairInfo[]): string[] {
  const tokens = new Set<string>()
  for (const pair of pairs) {
    tokens.add(assetInfoLabel(pair.asset_infos[0]))
    tokens.add(assetInfoLabel(pair.asset_infos[1]))
  }

  for (const [nativeDenom, wrappedAddr] of Object.entries(NATIVE_WRAPPED_PAIRS)) {
    if (wrappedAddr && tokens.has(wrappedAddr)) {
      tokens.add(nativeDenom)
    }
  }

  return Array.from(tokens)
}

/**
 * Detect a direct 1:1 wrap (native -> wrapped CW20) or unwrap (wrapped CW20 -> native).
 */
export function isDirectWrapUnwrap(fromToken: string, toToken: string): 'wrap' | 'unwrap' | null {
  if (isNativeDenom(fromToken) && getWrappedEquivalent(fromToken) === toToken) return 'wrap'
  if (isNativeDenom(toToken) && getWrappedEquivalent(toToken) === fromToken) return 'unwrap'
  return null
}

/**
 * Find a route, substituting native denoms with their wrapped CW20 equivalents
 * so the BFS can traverse the pair graph.
 */
export function findRouteWithNativeSupport(
  pairs: PairInfo[],
  fromToken: string,
  toToken: string
): { operations: SwapOperation[]; needsWrapInput: boolean; needsUnwrapOutput: boolean } | null {
  const direct = isDirectWrapUnwrap(fromToken, toToken)
  if (direct) return null

  const effectiveFrom = isNativeDenom(fromToken) ? (getWrappedEquivalent(fromToken) ?? fromToken) : fromToken
  const effectiveTo = isNativeDenom(toToken) ? (getWrappedEquivalent(toToken) ?? toToken) : toToken

  const route = findRoute(pairs, effectiveFrom, effectiveTo)
  if (!route) return null

  return {
    operations: route,
    needsWrapInput: isNativeDenom(fromToken),
    needsUnwrapOutput: isNativeDenom(toToken),
  }
}

/**
 * Simulate a swap that may involve native tokens by substituting with wrapped equivalents.
 */
export async function simulateNativeSwap(
  offerAmount: string,
  fromToken: string,
  toToken: string,
  pairs: PairInfo[]
): Promise<{ amount: string; isDirectWrapUnwrap: boolean }> {
  const direct = isDirectWrapUnwrap(fromToken, toToken)
  if (direct) {
    return { amount: offerAmount, isDirectWrapUnwrap: true }
  }

  const routeInfo = findRouteWithNativeSupport(pairs, fromToken, toToken)
  if (!routeInfo) {
    throw new Error('No route found')
  }

  const result = await simulateMultiHopSwap(offerAmount, routeInfo.operations)
  return { amount: result.amount, isDirectWrapUnwrap: false }
}

/**
 * Execute a swap involving native tokens via multi-message transactions.
 *
 * Flow variants:
 * - Direct wrap: single MsgExecuteContract to treasury WrapDeposit
 * - Direct unwrap: CW20 Send to wrap-mapper with Unwrap hook
 * - Native input swap: Msg1 = WrapDeposit, Msg2 = CW20 Send to router
 * - Native output swap: CW20 Send to router with unwrap_output: true
 * - Native-to-native: Msg1 = WrapDeposit, Msg2 = CW20 Send to router (unwrap_output: true)
 */
export async function executeNativeSwap(
  walletAddress: string,
  fromToken: string,
  toToken: string,
  amount: string,
  pairs: PairInfo[],
  maxSpread: string,
  minimumReceive?: string,
  deadline?: number
): Promise<string> {
  const direct = isDirectWrapUnwrap(fromToken, toToken)

  if (direct === 'wrap') {
    return executeTerraContract(walletAddress, TREASURY_CONTRACT_ADDRESS, { wrap_deposit: {} }, [
      { denom: fromToken, amount },
    ])
  }

  if (direct === 'unwrap') {
    const unwrapMsg = btoa(JSON.stringify({ unwrap: { recipient: null } }))
    return executeTerraContract(walletAddress, fromToken, {
      send: {
        contract: WRAP_MAPPER_CONTRACT_ADDRESS,
        amount,
        msg: unwrapMsg,
      },
    })
  }

  const routeInfo = findRouteWithNativeSupport(pairs, fromToken, toToken)
  if (!routeInfo) throw new Error('No route found')

  const needsWrap = routeInfo.needsWrapInput
  const needsUnwrap = routeInfo.needsUnwrapOutput
  const wrappedInput = needsWrap ? getWrappedEquivalent(fromToken)! : fromToken

  const swapHookMsg = {
    execute_swap_operations: {
      operations: routeInfo.operations.map((op) => ({ terra_swap: op.terra_swap })),
      max_spread: maxSpread,
      minimum_receive: minimumReceive,
      to: undefined,
      deadline,
      unwrap_output: needsUnwrap ? true : undefined,
    },
  }

  const sendToRouterMsg = {
    send: {
      contract: ROUTER_CONTRACT_ADDRESS,
      amount,
      msg: btoa(JSON.stringify(swapHookMsg)),
    },
  }

  if (needsWrap) {
    return executeTerraContractMulti(walletAddress, [
      {
        contract: TREASURY_CONTRACT_ADDRESS,
        msg: { wrap_deposit: {} },
        coins: [{ denom: fromToken, amount }],
      },
      {
        contract: wrappedInput,
        msg: sendToRouterMsg,
      },
    ])
  }

  return executeTerraContract(walletAddress, fromToken, sendToRouterMsg)
}
