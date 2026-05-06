import type { LimitOrderEscrowPlaceGateResult } from '@/utils/limitOrderEscrowBalanceGate'

type Props = {
  gate: LimitOrderEscrowPlaceGateResult
  /** Optional hook for Playwright / RTL */
  'data-testid'?: string
}

/**
 * Inline status for limit placement when escrow balance, native gas balance, or load state blocks
 * before submit. Pair with `evaluateLimitOrderEscrowPlaceGate` and `evaluateLimitOrderNativeGasPlaceGate`.
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
