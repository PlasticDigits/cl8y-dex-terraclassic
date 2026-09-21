/**
 * Native wrap Swap hints ([GitLab #587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587),
 * [Forgejo #1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264)).
 *
 * `uluna` and `uusd` both wrap into CW20, but **only `uluna` pays the network fee
 * from the pay balance**. Mixing those in SwapPage treated micro-uusd as
 * micro-uluna for Max and the LUNC gas gate.
 *
 * Invariants **G1264-4 / G1264-5**.
 */

/** Bank denom that actually pays Terra Classic fees. */
export function isNativeUlunaDenom(tokenId: string): boolean {
  return tokenId === 'uluna'
}

/**
 * Hub wrap+swap is typically 2 router hops (cLUNC/cUSTC → UST1 → USTR).
 * Until client-BFS `operations` exist, default to **2** so the Network-fee
 * hint includes `WRAP_ROUTER_COMBO_OVERHEAD_GAS` instead of wrap+1hop 1.8M.
 */
export function defaultNativeWrapHopCount(args: {
  operationsLength?: number | null
  payIsNativeDenom: boolean
  isDirectWrapOrUnwrap: boolean
}): number {
  const n = args.operationsLength
  if (n != null && n > 0) return n
  return args.payIsNativeDenom && !args.isDirectWrapOrUnwrap ? 2 : 1
}

/** Wrap the native pay token unless this is a direct wrap/unwrap or the route says otherwise. */
export function defaultNativeNeedsWrapInput(args: {
  routeNeedsWrapInput?: boolean | null
  payIsNativeDenom: boolean
  isDirectWrapOrUnwrap: boolean
}): boolean {
  if (args.routeNeedsWrapInput != null) return args.routeNeedsWrapInput
  return args.payIsNativeDenom && !args.isDirectWrapOrUnwrap
}
