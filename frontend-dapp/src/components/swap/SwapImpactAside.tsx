import { impactAsideOutsideDetails } from '@/utils/swapRouteSlippage'

/**
 * One aggregate fair-reference percentage and bounded explanation in both entry flows.
 * Per-hop spreads remain separate diagnostic metrics, never an adjacent percentage.
 */
export function SwapImpactAside({
  routeSlippagePct,
  worstHopPercent,
  economicExplanation,
  testId = 'swap-impact-aside',
}: {
  routeSlippagePct: number | null
  worstHopPercent: string | null | undefined
  economicExplanation?: string
  testId?: string
}) {
  const aside = impactAsideOutsideDetails({ routeSlippagePct, worstHopPercent })
  if (!aside) return null
  if (aside.kind === 'route') {
    return (
      <p className="mb-3 text-[11px] sm:text-xs" style={{ color: 'var(--ink-dim)' }} data-testid={testId}>
        Fair-reference deviation {aside.percent}%. {economicExplanation ?? 'Cost breakdown unavailable.'}
      </p>
    )
  }
  return (
    <p className="card-glass mb-3 text-[11px] sm:text-xs" style={{ color: 'var(--ink-dim)' }} data-testid={testId}>
      Worst hop spread ≈ {aside.percent}%.{' '}
      <a
        href="https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/swap-max-spread-ux.md"
        target="_blank"
        rel="noopener noreferrer"
        className="underline"
      >
        Docs
      </a>
    </p>
  )
}
