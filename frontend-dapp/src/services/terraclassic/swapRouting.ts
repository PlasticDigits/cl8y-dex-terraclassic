import type { HybridSwapParams } from '@/types'
import type { SwapOperation } from './router'

/**
 * Router `execute_swap_operations` is only for **≥ 2 hops** (SubMsg/reply overhead per hop).
 * Single-hop with a known pair uses CW20 `send` → pair `swap` (GitLab #249).
 */
export function swapOpsRequireRouter(ops: SwapOperation[] | undefined): boolean {
  return (ops?.length ?? 0) >= 2
}

export function hybridFromSingleHopIndexerOps(ops: SwapOperation[] | undefined): HybridSwapParams | undefined {
  if (!ops || ops.length !== 1) return undefined
  return ops[0].terra_swap.hybrid ?? undefined
}
