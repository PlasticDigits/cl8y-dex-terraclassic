import { getWrappedEquivalent, isNativeDenom } from '@/types'
import { isDirectWrapUnwrap, type SwapOperation } from '@/services/terraclassic/router'
import { stripAllDeclaredHybrid } from '@/utils/hybridHopOfferPartition'

/**
 * Native wrap-enter / unwrap-exit mapping for GET `/route/solve` (#1218).
 *
 * Maps the graph endpoints for route availability/display only. Quotes and progress
 * keep the original native identity and gross amount; the indexer owns wrapper
 * credit and policy admission (#1489). Wrapping consumes no pool-hop budget.
 * Direct 1:1 wrap/unwrap has no pool-route mapping.
 */

export type WrapMappedSolvePair = {
  tokenIn: string
  tokenOut: string
  needsWrapInput: boolean
  needsUnwrapOutput: boolean
}

/** Map native denoms to wrap-mapper CW20s. `null` for CW20↔CW20 or direct 1:1 wrap/unwrap. */
export function wrapMappedSolvePair(fromToken: string, toToken: string): WrapMappedSolvePair | null {
  if (!fromToken || !toToken) return null
  if (isDirectWrapUnwrap(fromToken, toToken)) return null
  const needsWrapInput = isNativeDenom(fromToken)
  const needsUnwrapOutput = isNativeDenom(toToken)
  if (!needsWrapInput && !needsUnwrapOutput) return null
  const tokenIn = needsWrapInput ? getWrappedEquivalent(fromToken) : fromToken
  const tokenOut = needsUnwrapOutput ? getWrappedEquivalent(toToken) : toToken
  if (!tokenIn?.startsWith('terra1') || !tokenOut?.startsWith('terra1')) return null
  return { tokenIn, tokenOut, needsWrapInput, needsUnwrapOutput }
}

/** Pool-only hops for wrap+N-hop execute (**H596-7** / **H1218-4**). Never copy `hybrid` / `book_input`. */
export function poolOnlyNativeExecuteOps(ops: SwapOperation[]): SwapOperation[] {
  return stripAllDeclaredHybrid(ops)
}
