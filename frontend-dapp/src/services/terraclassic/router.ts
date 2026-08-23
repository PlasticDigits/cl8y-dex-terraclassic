import { queryContract } from './queries'
import { executeTerraContract, executeTerraContractMulti } from './transactions'
import type { QuoteTraderOptions } from './pair'
import {
  ROUTER_CONTRACT_ADDRESS,
  NATIVE_WRAPPED_PAIRS,
  TREASURY_CONTRACT_ADDRESS,
  WRAP_MAPPER_CONTRACT_ADDRESS,
} from '@/utils/constants'
import type { AssetInfo, HybridSwapParams, PairInfo } from '@/types'
import { tokenAssetInfo, assetInfoLabel, isNativeDenom, getWrappedEquivalent } from '@/types'
import { netUlunaAfterTransferTaxAsync } from '@/utils/nativeTransferTax'
import {
  isGemTokenId,
  isTestPair,
  pairInfoLegIds,
  pairInfoLegSymbols,
  retailExposeTestTokens,
} from '@/utils/pairCatalogRank'
import { netAfterWrapMapperFee, queryWrapMapperFeeBps } from './wrapMapper'

/** Result of `simulateNativeSwap` (direct wrap/unwrap + native-routed swaps). */
export type NativeSwapSimResult = {
  /**
   * Expected amount the user receives.
   * Unwrap / native-output: after mapper fee **and** Classic burn tax on InstantWithdraw (#512).
   * Wrap / wrap-input: after mapper fee only (`MsgExecuteContract` funds are untaxed).
   */
  amount: string
  /**
   * Base for router `minimum_receive` (R3): post–mapper-fee, **pre–burn-tax**.
   * Equals `amount` when no unwrap burn tax applies. Always use this (not `amount`) when
   * submitting `minimum_receive` on `unwrap_output` paths — the router checks fee-net only.
   */
  routerMinReceiveBase: string
  isDirectWrapUnwrap: boolean
}

export interface SwapOperation {
  terra_swap: {
    offer_asset_info: AssetInfo
    ask_asset_info: AssetInfo
    hybrid?: HybridSwapParams | null
    /** Per-hop floor when `book_input > 0` without `belief_price` (GitLab #334). */
    min_return?: string | null
  }
}

export function serializeTerraSwap(ts: SwapOperation['terra_swap']) {
  const out: Record<string, unknown> = {
    offer_asset_info: ts.offer_asset_info,
    ask_asset_info: ts.ask_asset_info,
  }
  if (ts.hybrid) {
    out.hybrid = {
      pool_input: ts.hybrid.pool_input,
      book_input: ts.hybrid.book_input,
      max_maker_fills: ts.hybrid.max_maker_fills,
      book_start_hint: ts.hybrid.book_start_hint ?? undefined,
    }
  }
  if (ts.min_return != null && ts.min_return !== '') {
    out.min_return = ts.min_return
  }
  return out
}

interface SimulateResponse {
  amount: string
}

export async function simulateMultiHopSwap(
  offerAmount: string,
  operations: SwapOperation[],
  quoteTrader?: QuoteTraderOptions
): Promise<SimulateResponse> {
  const msg: Record<string, unknown> = {
    offer_amount: offerAmount,
    operations: operations.map((op) => ({ terra_swap: serializeTerraSwap(op.terra_swap) })),
  }
  if (quoteTrader?.trader) msg.trader = quoteTrader.trader
  if (quoteTrader?.sender) msg.sender = quoteTrader.sender
  return queryContract<SimulateResponse>(ROUTER_CONTRACT_ADDRESS, {
    simulate_swap_operations: msg,
  })
}

export async function reverseSimulateMultiHopSwap(
  askAmount: string,
  operations: SwapOperation[]
): Promise<SimulateResponse> {
  return queryContract<SimulateResponse>(ROUTER_CONTRACT_ADDRESS, {
    reverse_simulate_swap_operations: {
      ask_amount: askAmount,
      operations: operations.map((op) => ({ terra_swap: serializeTerraSwap(op.terra_swap) })),
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
        operations: operations.map((op) => ({ terra_swap: serializeTerraSwap(op.terra_swap) })),
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
function routingGraphPairs(pairs: PairInfo[], fromToken: string, toToken: string): PairInfo[] {
  if (retailExposeTestTokens()) return pairs
  if (isGemTokenId(fromToken) || isGemTokenId(toToken)) return pairs
  return pairs.filter((pair) => {
    const [id0, id1] = pairInfoLegIds(pair)
    const [s0, s1] = pairInfoLegSymbols(pair)
    return !isTestPair(s0, s1, id0, id1)
  })
}

export function findRoute(pairs: PairInfo[], fromToken: string, toToken: string): SwapOperation[] | null {
  if (fromToken === toToken) return null

  const graph = new Map<string, { token: string; pair: PairInfo }[]>()

  for (const pair of routingGraphPairs(pairs, fromToken, toToken)) {
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
 * Extract all unique tokens from the pairs list.
 * When wrap env is set, always include native LUNC/USTC and cLUNC/cUSTC so Swap can
 * direct-wrap without requiring a factory pair that already lists the wrapped CW20
 * (mainnet soft-launch has gemstone pairs only — GitLab #502 / #507).
 */
export function getAllTokens(pairs: PairInfo[]): string[] {
  const tokens = new Set<string>()
  for (const pair of pairs) {
    tokens.add(assetInfoLabel(pair.asset_infos[0]))
    tokens.add(assetInfoLabel(pair.asset_infos[1]))
  }

  for (const [nativeDenom, wrappedAddr] of Object.entries(NATIVE_WRAPPED_PAIRS)) {
    if (!wrappedAddr) continue
    tokens.add(wrappedAddr)
    tokens.add(nativeDenom)
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
 * CW20 amount minted for a gross native wrap deposit: wrap-mapper `fee_wrap_bps` only.
 *
 * Classic does **not** burn-tax `MsgExecuteContract` funds (user → treasury wrap_deposit),
 * so mint = `gross − floor(gross × fee_wrap_bps / 10_000)` (GitLab #512 / #516).
 * Must match `executeNativeSwap` / pool auto-wrap send amounts (#507).
 */
export async function netCw20AfterNativeWrap(grossNative: bigint, denom?: string): Promise<bigint> {
  void denom // retained for call-site clarity (native denom of the wrap_deposit)
  const feeBps = await queryWrapMapperFeeBps('wrap')
  return netAfterWrapMapperFee(grossNative, feeBps)
}

/**
 * Native amount a user receives after unwrap: `fee_unwrap_bps` then InstantWithdraw burn tax
 * (#512 / #516). `routerMinReceiveBase` is the post-fee pre-tax amount (R3).
 */
export async function netNativeAfterUnwrap(
  wrappedAmount: bigint,
  nativeDenom: string
): Promise<{ receive: bigint; routerMinReceiveBase: bigint }> {
  const feeBps = await queryWrapMapperFeeBps('unwrap')
  const afterFee = netAfterWrapMapperFee(wrappedAmount, feeBps)
  const receive = await netUlunaAfterTransferTaxAsync(afterFee, nativeDenom)
  return { receive, routerMinReceiveBase: afterFee }
}

/**
 * Simulate a swap that may involve native tokens by substituting with wrapped equivalents.
 * Direct wrap/unwrap and unwrap_output paths apply on-chain wrap / unwrap mapper fees (#507 / #516)
 * and Classic burn tax on unwrap InstantWithdraw (#512).
 */
export async function simulateNativeSwap(
  offerAmount: string,
  fromToken: string,
  toToken: string,
  pairs: PairInfo[]
): Promise<NativeSwapSimResult> {
  const direct = isDirectWrapUnwrap(fromToken, toToken)
  if (direct === 'wrap') {
    const net = await netCw20AfterNativeWrap(BigInt(offerAmount), fromToken)
    const amount = net.toString()
    return { amount, routerMinReceiveBase: amount, isDirectWrapUnwrap: true }
  }
  if (direct === 'unwrap') {
    const { receive, routerMinReceiveBase } = await netNativeAfterUnwrap(BigInt(offerAmount), toToken)
    return {
      amount: receive.toString(),
      routerMinReceiveBase: routerMinReceiveBase.toString(),
      isDirectWrapUnwrap: true,
    }
  }

  const routeInfo = findRouteWithNativeSupport(pairs, fromToken, toToken)
  if (!routeInfo) {
    throw new Error('No route found')
  }

  let simOffer = offerAmount
  if (routeInfo.needsWrapInput) {
    simOffer = (await netCw20AfterNativeWrap(BigInt(offerAmount), fromToken)).toString()
  }

  const result = await simulateMultiHopSwap(simOffer, routeInfo.operations)
  if (routeInfo.needsUnwrapOutput) {
    const { receive, routerMinReceiveBase } = await netNativeAfterUnwrap(BigInt(result.amount), toToken)
    return {
      amount: receive.toString(),
      routerMinReceiveBase: routerMinReceiveBase.toString(),
      isDirectWrapUnwrap: false,
    }
  }
  return {
    amount: result.amount,
    routerMinReceiveBase: result.amount,
    isDirectWrapUnwrap: false,
  }
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

  let cw20SendAmount = amount
  if (needsWrap) {
    // Gross native deposit; CW20 send must use post–wrap-fee mint (#507 / #512).
    cw20SendAmount = (await netCw20AfterNativeWrap(BigInt(amount), fromToken)).toString()
  }

  // Pool-only hops from client BFS `findRoute` — never copy hybrid / book_input (#587).
  const swapHookMsg = {
    execute_swap_operations: {
      operations: routeInfo.operations.map((op) => ({
        terra_swap: serializeTerraSwap({
          offer_asset_info: op.terra_swap.offer_asset_info,
          ask_asset_info: op.terra_swap.ask_asset_info,
          min_return: op.terra_swap.min_return,
        }),
      })),
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
      amount: cw20SendAmount,
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
