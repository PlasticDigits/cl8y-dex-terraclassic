import type { AssetInfo } from '@/types'
import { SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT, SWAP_EXTREME_SLIPPAGE_WARNING_PCT } from '@/utils/swapRouteSlippage'
import { lookupByTokenId, USDT_CW20_ADDRESS, USTR_CW20_ADDRESS } from '@/utils/tokenRegistry'

/**
 * Human-scale decimals for Swap / Trade quote display (#1257).
 *
 * Registry USTR / USDT are 18. Unknown CW20 still defaults to 6 (#1255 dummy tokens).
 * Pinning the two columbus-5 addresses here means a missing TOKENS row cannot reprint
 * 18-dec raw as billions.
 */
export function swapAmountDecimals(tokenId: string | undefined | null): number {
  if (!tokenId?.trim()) return 6
  const id = tokenId.trim().toLowerCase()
  if (id === USTR_CW20_ADDRESS.toLowerCase() || id === USDT_CW20_ADDRESS.toLowerCase()) return 18
  return lookupByTokenId(tokenId)?.decimals ?? 6
}

export function swapAmountDecimalsFromAssetInfo(info: AssetInfo): number {
  if ('token' in info) return swapAmountDecimals(info.token.contract_addr)
  return lookupByTokenId(info.native_token.denom)?.decimals ?? 6
}

/** ≥99% expected slippage vs best-route spot — screenshot theater, not a fillable quote. */
export function isTheaterRouteQuote(expectedSlippagePct: number | null | undefined): boolean {
  return expectedSlippagePct != null && expectedSlippagePct >= SWAP_EXTREME_SLIPPAGE_WARNING_PCT
}

/**
 * Retail submit guard: >30% needs Expert Mode; ≥99% never submits (Expert does not waive).
 */
export function swapRouteSlippageBlocksSubmit(
  expectedSlippagePct: number | null | undefined,
  expertMode: boolean
): boolean {
  if (expectedSlippagePct == null) return false
  if (isTheaterRouteQuote(expectedSlippagePct)) return true
  return expectedSlippagePct > SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT && !expertMode
}
