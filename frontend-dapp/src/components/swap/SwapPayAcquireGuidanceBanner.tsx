import { Link } from 'react-router-dom'
import { isAllowedAcquireHref, type SwapPayAcquireGuidance } from '@/utils/swapPayAcquireGuidance'

/**
 * One-sentence shortfall / funded size warning + Guide / Reduce controls (GitLab #678).
 * Guide `to` is allowlisted in the helper — never built from token symbols.
 */
export function SwapPayAcquireGuidanceBanner({
  guidance,
  onReduce,
  testIdPrefix,
}: {
  guidance: SwapPayAcquireGuidance
  onReduce?: (human: string) => void
  testIdPrefix: 'swap' | 'trade-market'
}) {
  if (guidance.kind === 'ok' || guidance.kind === 'disconnected_quote') return null
  if (!guidance.message) return null

  const href = guidance.guideHref && isAllowedAcquireHref(guidance.guideHref) ? guidance.guideHref : null

  return (
    <div className="alert-error mb-3 text-xs space-y-2" role="alert" data-testid={`${testIdPrefix}-acquire-guidance`}>
      <p data-testid={`${testIdPrefix}-acquire-message`}>{guidance.message}</p>
      {(href || (guidance.reduceToHuman && onReduce)) && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
          {href && guidance.guideLabel && (
            <Link to={href} className="underline font-semibold" data-testid={`${testIdPrefix}-acquire-guide`}>
              {guidance.guideLabel}
            </Link>
          )}
          {guidance.reduceToHuman && onReduce && (
            <button
              type="button"
              className="underline font-semibold"
              data-testid={`${testIdPrefix}-acquire-reduce`}
              onClick={() => onReduce(guidance.reduceToHuman!)}
            >
              Use {guidance.reduceToHuman} instead
            </button>
          )}
        </div>
      )}
    </div>
  )
}
