import { formatTokenAmount, toRawAmount } from '@/utils/formatAmount'
import type { EscrowBalanceQueryLike, LimitOrderEscrowPlaceGateTone } from '@/utils/limitOrderEscrowBalanceGate'

export const PROVIDE_LIQ_NATIVE_GAS_MSG_LOADING = 'Loading LUNC balance…'
export const PROVIDE_LIQ_NATIVE_GAS_MSG_UNAVAILABLE =
  'Cannot verify LUNC balance. Wait and retry, or check your connection.'

export function provideLiquidityCw20NativeGasInsufficientMessage(requiredUluna: bigint): string {
  const approx = formatTokenAmount(requiredUluna.toString(), 6, 5)
  return `Not enough LUNC for gas. You need at least ~${approx} LUNC to cover all three transactions (allowance A + allowance B + provide liquidity).`
}

export type ProvideLiquidityNativeGasGateResult = {
  /** When false, the UI must not call `provideLiquidity` for the CW20/CW20 path. */
  canAddLiquidity: boolean
  userMessage: string | null
  tone: LimitOrderEscrowPlaceGateTone
}

/**
 * Pre-flight gate: bank **uluna** must cover **three** separate txs (`increase_allowance` ×2 then
 * `provide_liquidity`) before the first broadcast on the CW20/CW20 path
 * ([GitLab #147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)).
 *
 * **Invariants**
 *
 * 1. When either human amount is empty / both raw amounts are zero, the gate is **open** (no message) —
 *    other controls already block submit; avoids noisy “add LUNC” before the user enters amounts.
 * 2. While LUNC balance is **loading** or **unreadable**, the gate is **closed** (conservative).
 * 3. Compare **BigInt** uluna balance to `requiredUluna` (sum of three `Fee.amount` estimates).
 * 4. Callers must pass `requiredUluna` from `estimateProvideLiquidityCw20SequenceUlunaFeesTotal()` in
 *    `transactions.ts` so UI preflight matches broadcast fee math.
 */
export function evaluateProvideLiquidityCw20NativeGasGate(
  amountHumanA: string,
  amountHumanB: string,
  decimalsA: number,
  decimalsB: number,
  ulunaQuery: EscrowBalanceQueryLike,
  requiredUluna: bigint
): ProvideLiquidityNativeGasGateResult {
  const rawA = toRawAmount(amountHumanA.trim(), decimalsA)
  const rawB = toRawAmount(amountHumanB.trim(), decimalsB)
  if (rawA === '0' || rawB === '0') {
    return { canAddLiquidity: true, userMessage: null, tone: 'none' }
  }

  if (ulunaQuery.isLoading) {
    return { canAddLiquidity: false, userMessage: PROVIDE_LIQ_NATIVE_GAS_MSG_LOADING, tone: 'warning' }
  }

  if (ulunaQuery.isError || ulunaQuery.data === undefined) {
    return {
      canAddLiquidity: false,
      userMessage: PROVIDE_LIQ_NATIVE_GAS_MSG_UNAVAILABLE,
      tone: 'error',
    }
  }

  let bal: bigint
  try {
    bal = BigInt(ulunaQuery.data)
  } catch {
    return {
      canAddLiquidity: false,
      userMessage: PROVIDE_LIQ_NATIVE_GAS_MSG_UNAVAILABLE,
      tone: 'error',
    }
  }

  if (bal < requiredUluna) {
    return {
      canAddLiquidity: false,
      userMessage: provideLiquidityCw20NativeGasInsufficientMessage(requiredUluna),
      tone: 'error',
    }
  }

  return { canAddLiquidity: true, userMessage: null, tone: 'none' }
}
