/**
 * Swap submit preflight: bank LUNC must cover network fee (and native pay + fee)
 * ([GitLab #587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587)).
 *
 * Same pattern as limit-place / provide-liquidity gates (#132 / #147).
 */
import { formatTokenAmount, toRawAmount } from '@/utils/formatAmount'
import type { EscrowBalanceQueryLike } from '@/utils/limitOrderEscrowBalanceGate'

export const SWAP_NATIVE_GAS_MSG_LOADING = 'Loading LUNC balance…'
export const SWAP_NATIVE_GAS_MSG_UNAVAILABLE = 'Cannot verify LUNC balance.'

export type SwapNativeGasGateResult = {
  canSubmit: boolean
  userMessage: string | null
  tone: 'none' | 'warning' | 'error'
}

export function swapNativeGasInsufficientMessage(requiredUluna: bigint): string {
  const approx = formatTokenAmount(requiredUluna.toString(), 6, 5)
  return `Need ~${approx} LUNC for network fee.`
}

/**
 * **Invariants**
 *
 * 1. Empty / zero pay amount → gate open (CTA already says Enter Amount).
 * 2. Loading or unreadable bank uluna → gate closed (conservative).
 * 3. Native LUNC pay: require `payRaw + feeUluna`. CW20 pay: require `feeUluna` only.
 * 4. `feeUluna` must come from {@link estimateSwapNetworkFee} / broadcast envelope.
 */
export function evaluateSwapNativeGasGate(
  amountHuman: string,
  payDecimals: number,
  payIsNativeUluna: boolean,
  payRaw: string,
  bankUlunaQuery: EscrowBalanceQueryLike,
  feeUluna: bigint
): SwapNativeGasGateResult {
  const raw = toRawAmount(amountHuman.trim(), payDecimals)
  if (raw === '0') {
    return { canSubmit: true, userMessage: null, tone: 'none' }
  }

  if (bankUlunaQuery.isLoading) {
    return { canSubmit: false, userMessage: SWAP_NATIVE_GAS_MSG_LOADING, tone: 'warning' }
  }

  if (bankUlunaQuery.isError || bankUlunaQuery.data === undefined) {
    return { canSubmit: false, userMessage: SWAP_NATIVE_GAS_MSG_UNAVAILABLE, tone: 'error' }
  }

  let bal: bigint
  try {
    bal = BigInt(bankUlunaQuery.data)
  } catch {
    return { canSubmit: false, userMessage: SWAP_NATIVE_GAS_MSG_UNAVAILABLE, tone: 'error' }
  }

  let pay: bigint
  try {
    pay = payIsNativeUluna ? BigInt(payRaw || '0') : 0n
  } catch {
    pay = 0n
  }

  const required = pay + feeUluna
  if (bal < required) {
    return {
      canSubmit: false,
      userMessage: swapNativeGasInsufficientMessage(feeUluna),
      tone: 'error',
    }
  }

  return { canSubmit: true, userMessage: null, tone: 'none' }
}
