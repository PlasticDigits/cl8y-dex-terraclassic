import { formatTokenAmount, toRawAmount } from '@/utils/formatAmount'
import type { EscrowBalanceQueryLike, LimitOrderEscrowPlaceGateResult } from '@/utils/limitOrderEscrowBalanceGate'

export const LIMIT_ORDER_NATIVE_GAS_MSG_LOADING = 'Loading LUNC balance…'
export const LIMIT_ORDER_NATIVE_GAS_MSG_UNAVAILABLE =
  'Cannot verify LUNC balance. Wait and retry, or check your connection.'

export function limitOrderNativeGasInsufficientMessage(requiredUluna: bigint): string {
  const approx = formatTokenAmount(requiredUluna.toString(), 6, 5)
  return `Not enough LUNC for both transactions. You need at least ~${approx} LUNC for gas (allowance + place order).`
}

/**
 * Pre-flight gate: native LUNC (uluna) must cover **both** txs in the CW20 limit place sequence
 * (`increase_allowance` then `place_limit_order`) before the first broadcast
 * ([GitLab #132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)).
 *
 * **Invariants**
 *
 * 1. When the human amount is empty / zero raw, the gate is **open** (no message) — the escrow gate
 *    already blocks submit; avoids noisy “add LUNC” before the user enters an amount.
 * 2. While LUNC balance is **loading** or **unreadable**, the gate is **closed** (conservative).
 * 3. Compare **BigInt** uluna balance to `requiredUluna` (sum of fee amounts the dApp attaches per tx).
 * 4. Callers must pass `requiredUluna` from `estimateLimitOrderPlaceSequenceUlunaFeesTotal()` in
 *    `transactions.ts` so UI preflight matches broadcast fee math.
 */
export function evaluateLimitOrderNativeGasPlaceGate(
  amountHuman: string,
  escrowDecimals: number,
  q: EscrowBalanceQueryLike,
  requiredUluna: bigint
): LimitOrderEscrowPlaceGateResult {
  const raw = toRawAmount(amountHuman.trim(), escrowDecimals)
  if (raw === '0') {
    return { canPlaceLimit: true, userMessage: null, tone: 'none' }
  }

  if (q.isLoading) {
    return { canPlaceLimit: false, userMessage: LIMIT_ORDER_NATIVE_GAS_MSG_LOADING, tone: 'warning' }
  }

  if (q.isError || q.data === undefined) {
    return {
      canPlaceLimit: false,
      userMessage: LIMIT_ORDER_NATIVE_GAS_MSG_UNAVAILABLE,
      tone: 'error',
    }
  }

  let bal: bigint
  try {
    bal = BigInt(q.data)
  } catch {
    return {
      canPlaceLimit: false,
      userMessage: LIMIT_ORDER_NATIVE_GAS_MSG_UNAVAILABLE,
      tone: 'error',
    }
  }

  if (bal < requiredUluna) {
    return {
      canPlaceLimit: false,
      userMessage: limitOrderNativeGasInsufficientMessage(requiredUluna),
      tone: 'error',
    }
  }

  return { canPlaceLimit: true, userMessage: null, tone: 'none' }
}
