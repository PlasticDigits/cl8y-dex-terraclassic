import type { LimitOrderEscrowPlaceGateResult } from '@/utils/limitOrderEscrowBalanceGate'
import type { LimitOrderPricePlaceGateResult } from '@/utils/limitOrderPricePlaceGate'

type Props = {
  gate: LimitOrderEscrowPlaceGateResult | LimitOrderPricePlaceGateResult
  /** Optional hook for Playwright / RTL */
  'data-testid'?: string
}

/**
 * Inline status for limit placement when escrow balance, native gas balance, price vs tape reference,
 * or load state blocks before submit. Pair with `evaluateLimitOrderEscrowPlaceGate`,
 * `evaluateLimitOrderNativeGasPlaceGate`, and `evaluateLimitOrderPricePlaceGate`.
 */
export function LimitOrderEscrowPlaceGuardMessage({ gate, 'data-testid': testId }: Props) {
  if (!gate.userMessage) return null

  if (gate.tone === 'warning') {
    return (
      <p className="text-xs" style={{ color: 'var(--ink-dim)' }} role="status" data-testid={testId}>
        {gate.userMessage}
      </p>
    )
  }

  return (
    <div className="alert-error text-xs" role="alert" data-testid={testId}>
      {gate.userMessage}
    </div>
  )
}
