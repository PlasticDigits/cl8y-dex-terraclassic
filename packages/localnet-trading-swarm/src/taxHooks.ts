/**
 * CW20 hook builders for tax-aware swarm (GitLab #621).
 *
 * Pair-direct `Swap.trader` stays unset — token ignores spoof and extra-debits `from`.
 * Official-router hops set `trader` only on the hop the **router** forwards
 * (`execute_swap_operations`); bots must not put `trader` on pair-direct Send.
 */

export interface PairDirectSwapHook {
  swap: {
    belief_price: undefined
    max_spread: string
    to: undefined
    deadline: undefined
    trader: undefined
    hybrid: undefined
  }
}

export function pairDirectSwapHook(maxSpread = '1'): PairDirectSwapHook {
  return {
    swap: {
      belief_price: undefined,
      max_spread: maxSpread,
      to: undefined,
      deadline: undefined,
      trader: undefined,
      hybrid: undefined,
    },
  }
}

/** Assert helper: pair-direct hook must not spoof another wallet. */
export function pairDirectSwapSetsTrader(hook: { swap?: { trader?: unknown } }): boolean {
  const t = hook.swap?.trader
  return t != null && t !== '' && t !== undefined
}

export function routerExecuteSwapOperations(ops: unknown[], maxSpread = '1'): Record<string, unknown> {
  return {
    execute_swap_operations: {
      operations: ops,
      max_spread: maxSpread,
      minimum_receive: undefined,
      to: undefined,
      deadline: undefined,
    },
  }
}

/**
 * Hop-side Swap the official router stamps with `trader: sender`.
 * Used only for TaxPreview of a router sell (from=router, to=listed pair).
 */
export function routerHopSwapPreviewHook(trader: string, maxSpread = '1'): Record<string, unknown> {
  return {
    swap: {
      belief_price: undefined,
      max_spread: maxSpread,
      to: undefined,
      deadline: undefined,
      trader,
      hybrid: undefined,
    },
  }
}
