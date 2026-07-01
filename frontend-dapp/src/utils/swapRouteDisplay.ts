import { assetInfoLabel } from '@/types'
import type { SwapOperation } from '@/services/terraclassic/router'
import { swapOpsRequireRouter } from '@/services/terraclassic/swapRouting'

/** Router / indexer multihop: token contract addresses in execution order (inclusive of ends). */
export function tokenPathFromSwapOperations(operations: SwapOperation[]): string[] {
  if (operations.length === 0) return []
  const out: string[] = [assetInfoLabel(operations[0].terra_swap.offer_asset_info)]
  for (const op of operations) {
    out.push(assetInfoLabel(op.terra_swap.ask_asset_info))
  }
  return out
}

/**
 * Cross-validate the intermediate token path shown to the user against the path actually
 * encoded in the operations that will be submitted (GitLab #450 / SEC-I02 H09).
 *
 * The route display uses `indexer.intermediate_tokens` while submit is built from
 * `router_operations`. Both come from the indexer with no cross-check, so a malicious or
 * compromised indexer could display one path and submit another within the user's slippage
 * tolerance. This re-derives the token path from the submitted operations and asserts it equals
 * the displayed `intermediate_tokens` (case-insensitive address compare, since addresses are the
 * trust anchor, not the display symbol). Any mismatch returns `false` so the caller can reject the
 * route before signing rather than trusting the indexer's display string.
 *
 * Returns `true` when there is nothing to cross-check (no operations, or no displayed
 * intermediate-token list) — those paths are validated elsewhere (`token_in`/`token_out` match in
 * `SwapPage`) and are not the spoofing surface this guard covers.
 */
export function swapRouteIntermediateTokensAligned(
  operations: SwapOperation[] | undefined,
  intermediateTokens: string[] | undefined
): boolean {
  if (!operations || operations.length === 0) return true
  if (!intermediateTokens || intermediateTokens.length === 0) return true

  const fromOps = tokenPathFromSwapOperations(operations)
  if (fromOps.length !== intermediateTokens.length) return false

  const norm = (addr: string) => addr.trim().toLowerCase()
  return fromOps.every((token, i) => norm(token) === norm(intermediateTokens[i]))
}

export interface SwapRouteIntermediateReconciliation {
  /** Token path used for display and submit (inclusive of ends). */
  tokens: string[]
  /** True when indexer `intermediate_tokens` disagreed with `router_operations`. */
  mismatch: boolean
}

/**
 * Reconcile indexer `intermediate_tokens` with the path encoded in `router_operations`
 * (GitLab #450 / SEC-I02 H09). Submit always follows operations; when the indexer display
 * path disagrees, the ops-derived path wins and `mismatch` is set so the UI can notify the user
 * instead of silently falling back to a different quote source.
 */
export function reconcileSwapRouteIntermediateTokens(
  operations: SwapOperation[] | undefined,
  intermediateTokens: string[] | undefined
): SwapRouteIntermediateReconciliation {
  if (!operations || operations.length === 0) {
    return { tokens: intermediateTokens ?? [], mismatch: false }
  }

  const fromOps = tokenPathFromSwapOperations(operations)
  if (!intermediateTokens || intermediateTokens.length === 0) {
    return { tokens: fromOps, mismatch: false }
  }

  if (swapRouteIntermediateTokensAligned(operations, intermediateTokens)) {
    return { tokens: intermediateTokens, mismatch: false }
  }

  return { tokens: fromOps, mismatch: true }
}

export interface NativeRouteForDisplay {
  operations: SwapOperation[]
  needsWrapInput: boolean
  needsUnwrapOutput: boolean
}

/**
 * Token path for native / wrapped swaps (matches legacy Swap page arrow semantics).
 * @param displaySymbol maps denom or CW20 address to a short UI label
 */
export function tokenPathForNativeSupportedRoute(
  fromToken: string,
  toToken: string,
  info: NativeRouteForDisplay,
  displaySymbol: (id: string) => string
): string[] {
  const parts: string[] = []
  if (info.needsWrapInput) parts.push(displaySymbol(fromToken))
  for (const op of info.operations) {
    parts.push(displaySymbol(assetInfoLabel(op.terra_swap.offer_asset_info)))
  }
  const last = info.operations[info.operations.length - 1]
  parts.push(displaySymbol(assetInfoLabel(last.terra_swap.ask_asset_info)))
  if (info.needsUnwrapOutput) parts.push(displaySymbol(toToken))
  return parts
}

export interface SwapRouteDisplayArgs {
  fromToken: string
  toToken: string
  isWrapOrUnwrap: boolean
  nativeRouteInfo: NativeRouteForDisplay | null
  /** Active quote: indexer-solved path token addresses (in order), when present. */
  indexerIntermediateTokens?: string[]
  /** Operations used when submitting via router (indexer-shaped). */
  indexerOperations?: SwapOperation[]
  /** Client BFS route when indexer ops are not used for submit. */
  clientRoute: SwapOperation[] | null
  isMultiHop: boolean
  isDirect: boolean
  displaySymbol: (id: string) => string
}

/**
 * Single execution-aligned route line for the Swap page trade summary.
 *
 * **Invariant (GitLab #158):** At most one route is shown; when the indexer supplies
 * `router_operations` used for submit, that path wins over the client `findRoute` graph
 * so users never see two identical arrows labeled differently.
 */
export function computeSwapRouteDisplay(args: SwapRouteDisplayArgs): string | null {
  const {
    fromToken,
    toToken,
    isWrapOrUnwrap,
    nativeRouteInfo,
    indexerIntermediateTokens,
    indexerOperations,
    clientRoute,
    isMultiHop,
    isDirect,
    displaySymbol,
  } = args

  if (!fromToken || !toToken) return null

  if (isWrapOrUnwrap) {
    return `${displaySymbol(fromToken)} → ${displaySymbol(toToken)}`
  }

  if (nativeRouteInfo && nativeRouteInfo.operations.length > 0) {
    return tokenPathForNativeSupportedRoute(fromToken, toToken, nativeRouteInfo, displaySymbol).join(' → ')
  }

  const idxOps = indexerOperations
  if (idxOps && idxOps.length > 0) {
    if (indexerIntermediateTokens && indexerIntermediateTokens.length >= 2) {
      return indexerIntermediateTokens.map((t) => displaySymbol(t)).join(' → ')
    }
    return tokenPathFromSwapOperations(idxOps)
      .map((t) => displaySymbol(t))
      .join(' → ')
  }

  if (isMultiHop && clientRoute && clientRoute.length > 0) {
    return tokenPathFromSwapOperations(clientRoute)
      .map((t) => displaySymbol(t))
      .join(' → ')
  }

  if (isDirect) {
    return `${displaySymbol(fromToken)} → ${displaySymbol(toToken)}`
  }

  return null
}

/** Submit path for Swap `swapMutation` — must stay aligned with `SwapPage` mutation branches. */
export type SwapSubmitRouteSource = 'indexer' | 'client_bfs' | 'direct' | 'native_wrap'

export function deriveSwapSubmitRouteSource(args: {
  isWrapOrUnwrap: boolean
  nativeRouteInfo: NativeRouteForDisplay | null
  indexerOperations?: SwapOperation[]
  clientRoute: SwapOperation[] | null
  isDirect: boolean
  isMultiHop: boolean
}): SwapSubmitRouteSource | null {
  const { isWrapOrUnwrap, nativeRouteInfo, indexerOperations, clientRoute, isDirect, isMultiHop } = args

  if (isWrapOrUnwrap || nativeRouteInfo) return 'native_wrap'
  if (swapOpsRequireRouter(indexerOperations)) return 'indexer'
  if (!clientRoute) return null
  if (isDirect) return 'direct'
  if (isMultiHop) return 'client_bfs'
  return null
}

/**
 * Swap operations used to resolve factory pair addresses — same precedence as `swapMutation`
 * (native wrap → indexer router ops → client BFS / direct). Prevents pre-sign pair rows from
 * showing BFS shortest-path hops while submit executes indexer `router_operations` ([#449]).
 */
export function deriveSwapSubmitRouteOps(args: {
  nativeRouteInfo: NativeRouteForDisplay | null
  indexerOperations?: SwapOperation[]
  clientRoute: SwapOperation[] | null
}): SwapOperation[] | null | undefined {
  const { nativeRouteInfo, indexerOperations, clientRoute } = args
  if (nativeRouteInfo?.operations?.length) return nativeRouteInfo.operations
  if (swapOpsRequireRouter(indexerOperations)) return indexerOperations
  return clientRoute ?? indexerOperations ?? undefined
}

/** Brief label when submit uses client BFS multihop (GitLab #302 / #329). */
export const SWAP_CLIENT_BFS_FALLBACK_COPY = 'Route source: client graph (shortest path; not best execution).'

/** Shown when indexer `intermediate_tokens` disagreed with `router_operations` and the route row was updated (GitLab #450 / SEC-I02 H09). */
export const SWAP_ROUTE_INTERMEDIATE_RECONCILED_COPY = 'Route adjusted.'
