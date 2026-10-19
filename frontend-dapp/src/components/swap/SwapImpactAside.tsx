import { formatTradingPriceDifference } from '@/utils/swapRouteSlippage'
import { tradingRiskNotice, type TradingRisk } from '@/utils/tradingRisk'

/** One actionable price notice; optional low-risk information stays in signing details. */
export function SwapImpactAside({
  routeSlippagePct,
  risk,
  details = false,
  blocked,
  confirming = false,
  onCancel,
  onEnableExpert,
  testId = 'swap-impact-aside',
}: {
  routeSlippagePct: number | null
  risk?: TradingRisk
  worstHopPercent?: string | null
  economicExplanation?: string
  details?: boolean
  blocked?: 'expert' | 'extreme'
  confirming?: boolean
  onCancel?: () => void
  onEnableExpert?: () => void
  testId?: string
}) {
  if (details && (routeSlippagePct == null || !Number.isFinite(routeSlippagePct))) return null
  if (!details && (!risk || (risk.availability === 'available' && !risk.consentRequired))) return null
  const metric = details
    ? `Fair-reference deviation ${formatTradingPriceDifference(routeSlippagePct!)}%`
    : tradingRiskNotice(risk!)
  const sentence =
    risk?.availability === 'unavailable' && !details
      ? metric
      : blocked === 'extreme'
        ? `${metric} is too high.`
        : blocked === 'expert'
          ? `${metric} requires Expert Mode.`
          : `${metric}${!details && confirming ? '; click again to confirm' : ''}.`
  return (
    <div
      className={
        'swap-impact-notice ' +
        (details
          ? 'text-[11px] sm:text-xs'
          : blocked || risk?.availability === 'unavailable'
            ? 'alert-error mb-3 text-xs'
            : 'alert-warning mb-3 text-xs')
      }
      role={details ? undefined : 'status'}
      style={details ? { color: 'var(--ink-dim)' } : undefined}
      data-testid={testId}
    >
      <p>
        {sentence}{' '}
        {!details && risk?.availability === 'available' && blocked === 'expert' && onEnableExpert && (
          <button type="button" className="swap-inline-action underline" onClick={onEnableExpert}>
            Enable Expert Mode
          </button>
        )}
        {!details && !blocked && confirming && onCancel && (
          <button type="button" className="swap-inline-action underline" onClick={onCancel}>
            Cancel
          </button>
        )}
      </p>
    </div>
  )
}
