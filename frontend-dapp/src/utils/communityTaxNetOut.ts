/**
 * Catalog buy-split net for You Receive (GitLab #615).
 * Does **not** resize hop `amount_in` (H-01 / T592-1). Max extra-debit is sell-side only.
 */

import { COMMUNITY_TAX_BPS_DENOM } from '@/utils/communityTaxSku'

/** `raw - floor(raw * buyBps / 10000)` — matches on-chain TaxPreview buy credit. */
export function applyBuyTaxNet(rawOut: string, buyBps: number | null | undefined): string {
  if (!rawOut || !/^\d+$/.test(rawOut)) return rawOut
  const bps = buyBps == null ? 0 : Math.max(0, Math.floor(buyBps))
  if (bps <= 0) return rawOut
  try {
    const raw = BigInt(rawOut)
    const tax = (raw * BigInt(bps)) / BigInt(COMMUNITY_TAX_BPS_DENOM)
    return (raw - tax).toString()
  } catch {
    return rawOut
  }
}

/** Manager-directory exempt → 0. Unknown exempt (`null`/`undefined`) fail-closed (keep bps). */
export function effectiveBuyTaxBps(
  buyBps: number | null | undefined,
  managerExempt: boolean | null | undefined
): number | null {
  if (buyBps == null) return null
  if (managerExempt === true) return 0
  return buyBps
}

/**
 * Apply indexer `buy_tax_bps` to a wallet-authoritative raw out.
 * When bps is missing/0, display stays raw (Honest hops / ordinary CW20).
 */
export function displayReceiveNet(rawWalletOut: string, buyTaxBps: number | null | undefined): string {
  return applyBuyTaxNet(rawWalletOut, buyTaxBps)
}

/** Pair-direct fallback: You Receive is net; `executeAmountOut` stays pre-tax for min_return. */
export function withBuyTaxReceiveDisplay<T extends { return_amount: string; executeAmountOut?: string }>(
  quoted: T,
  buyBps: number | null | undefined
): T {
  const net = displayReceiveNet(quoted.return_amount, buyBps)
  if (net === quoted.return_amount) return quoted
  return { ...quoted, return_amount: net, executeAmountOut: quoted.executeAmountOut ?? quoted.return_amount }
}
