/**
 * Catalog buy-split net for You Receive (GitLab #615).
 * Legacy direct-pair display conversion only. Supported route-funded v1 router
 * simulations already include every buy/sell tax; do not apply this haircut again.
 */

import { COMMUNITY_TAX_BPS_DENOM } from '@/utils/communityTaxSku'

/**
 * Minimum pre-buy-tax ask for a typed net receive (#1256 / #615).
 * Does not gross up sell extra-debit or change execute min_return accounting.
 */
export function grossBuyTaxForTargetNet(rawNet: string, buyBps: number | null | undefined): string {
  if (!/^\d+$/.test(rawNet)) throw new Error('Invalid receive amount for reverse quote')
  const bps = buyBps ?? 0
  if (!Number.isInteger(bps) || bps < 0 || bps >= COMMUNITY_TAX_BPS_DENOM) {
    throw new Error('Invalid buy tax for reverse quote')
  }
  const target = BigInt(rawNet)
  if (target <= 0n) return '0'
  const denominator = BigInt(COMMUNITY_TAX_BPS_DENOM)
  return (((target - 1n) * denominator) / (denominator - BigInt(bps)) + 1n).toString()
}

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
export function withBuyTaxReceiveDisplay<
  T extends { return_amount: string; executeAmountOut?: string; taxFundingVersion?: 1 },
>(quoted: T, buyBps: number | null | undefined): T {
  if (quoted.taxFundingVersion === 1) return quoted
  const net = displayReceiveNet(quoted.return_amount, buyBps)
  if (net === quoted.return_amount) return quoted
  return { ...quoted, return_amount: net, executeAmountOut: quoted.executeAmountOut ?? quoted.return_amount }
}
