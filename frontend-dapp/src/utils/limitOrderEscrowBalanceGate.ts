import { toRawAmount } from '@/utils/formatAmount'

/** User-visible copy — keep in sync with `LimitOrderEscrowPlaceGuardMessage` styling rules. */
export const LIMIT_ORDER_ESCROW_MSG_INSUFFICIENT = 'Insufficient balance'
export const LIMIT_ORDER_ESCROW_MSG_LOADING = 'Loading wallet balance…'
export const LIMIT_ORDER_ESCROW_MSG_BALANCE_UNAVAILABLE =
  'Cannot verify wallet balance. Wait and retry, or check your connection.'

/**
 * Minimal balance-query shape so callers can pass React Query results without
 * importing `@tanstack/react-query` here.
 */
export type EscrowBalanceQueryLike = {
  data: string | undefined
  isLoading: boolean
  isError: boolean
}

export type LimitOrderEscrowPlaceGateTone = 'none' | 'warning' | 'error'

export type LimitOrderEscrowPlaceGateResult = {
  /** When false, the UI must not call `increase_allowance` / `place_limit_order`. */
  canPlaceLimit: boolean
  /** Inline hint under the Place control; null when there is nothing to show. */
  userMessage: string | null
  tone: LimitOrderEscrowPlaceGateTone
}

/**
 * Pre-flight gate for limit placement escrow (CW20 raw amount vs wallet balance).
 *
 * **Invariants**
 *
 * 1. **No on-chain txs** unless `canPlaceLimit` is true (callers should also disable the button).
 * 2. **BigInt compare** on raw micro-units — same basis as `toRawAmount` / `increase_allowance` amount.
 * 3. While balance is **loading** or **unreadable**, the gate is closed (conservative — avoids the
 *    “allowance succeeds, `transfer_from` fails” gas burn from [GitLab #130](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/130)).
 * 4. **Zero / empty human amount** → `canPlaceLimit: false` with no user message (existing “Enter amount”
 *    path on submit remains a safety net).
 */
export function evaluateLimitOrderEscrowPlaceGate(
  amountHuman: string,
  escrowDecimals: number,
  q: EscrowBalanceQueryLike
): LimitOrderEscrowPlaceGateResult {
  const raw = toRawAmount(amountHuman.trim(), escrowDecimals)

  if (raw === '0') {
    return { canPlaceLimit: false, userMessage: null, tone: 'none' }
  }

  if (q.isLoading) {
    return { canPlaceLimit: false, userMessage: LIMIT_ORDER_ESCROW_MSG_LOADING, tone: 'warning' }
  }

  if (q.isError || q.data === undefined) {
    return {
      canPlaceLimit: false,
      userMessage: LIMIT_ORDER_ESCROW_MSG_BALANCE_UNAVAILABLE,
      tone: 'error',
    }
  }

  let spend: bigint
  let bal: bigint
  try {
    spend = BigInt(raw)
    bal = BigInt(q.data)
  } catch {
    return {
      canPlaceLimit: false,
      userMessage: LIMIT_ORDER_ESCROW_MSG_BALANCE_UNAVAILABLE,
      tone: 'error',
    }
  }

  if (spend > bal) {
    return { canPlaceLimit: false, userMessage: LIMIT_ORDER_ESCROW_MSG_INSUFFICIENT, tone: 'error' }
  }

  return { canPlaceLimit: true, userMessage: null, tone: 'none' }
}
