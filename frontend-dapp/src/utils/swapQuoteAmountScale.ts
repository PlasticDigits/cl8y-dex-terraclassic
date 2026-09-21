import type { AssetInfo } from '@/types'
import { SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT, SWAP_EXTREME_SLIPPAGE_WARNING_PCT } from '@/utils/swapRouteSlippage'
import { lookupByTokenId, USDT_CW20_ADDRESS, USTR_CW20_ADDRESS } from '@/utils/tokenRegistry'

/**
 * Listed-token pin for USTR / USDT display (#1257).
 *
 * Swap / Trade **execute** amounts must use `resolveSwapAssetDecimals` /
 * `useAssetDecimals` (#1255). Unknown CW20 is **not** 6 on those surfaces.
 * This helper still defaults unknown to 6 for non-execute chrome that has not
 * switched yet (`getDecimals` class). Do not use it as Swap sim/execute scale.
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
