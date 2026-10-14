import { formatTradingPriceDifference } from '@/utils/swapRouteSlippage'
import { SWAP_FUNDED_HIGH_IMPACT_PCT } from '@/utils/swapPayAcquireGuidance'

/** One actionable price notice; optional low-risk information stays in signing details. */
export function SwapImpactAside({
  routeSlippagePct,
  details = false,
  blocked,
  confirming = false,
  onCancel,
  onEnableExpert,
  testId = 'swap-impact-aside',
}: {
  routeSlippagePct: number | null
  worstHopPercent?: string | null
  economicExplanation?: string
  details?: boolean
  blocked?: 'expert' | 'extreme'
  confirming?: boolean
  onCancel?: () => void
  onEnableExpert?: () => void
  testId?: string
}) {
  if (routeSlippagePct == null || !Number.isFinite(routeSlippagePct)) return null
  if (!details && routeSlippagePct <= SWAP_FUNDED_HIGH_IMPACT_PCT) return null
  const metric = `Fair-reference deviation ${formatTradingPriceDifference(routeSlippagePct)}%`
  const sentence =
    blocked === 'extreme'
      ? `${metric} is too high.`
      : blocked === 'expert'
        ? `${metric} requires Expert Mode.`
        : `${metric}${!details && confirming ? '; click again to confirm' : ''}.`
  return (
    <div
      className={details ? 'text-[11px] sm:text-xs' : 'alert-error mb-3 text-xs'}
      role={details ? undefined : 'alert'}
      style={details ? { color: 'var(--ink-dim)' } : undefined}
      data-testid={testId}
    >
      <p>{sentence}</p>
      {!details && blocked === 'expert' && onEnableExpert && (
        <button type="button" className="underline" onClick={onEnableExpert}>
          Enable Expert Mode
        </button>
      )}
      {!details && !blocked && confirming && onCancel && (
        <button type="button" className="underline" onClick={onCancel}>
          Cancel
        </button>
      )}
    </div>
  )
}
