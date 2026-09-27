import { impactAsideOutsideDetails } from '@/utils/swapRouteSlippage'

/**
 * One percent outside collapsed trade details (#1360).
 * High-impact route slippage replaces a different worst-hop line.
 */
export function SwapImpactAside({
  routeSlippagePct,
  worstHopPercent,
  testId = 'swap-impact-aside',
}: {
  routeSlippagePct: number | null
  worstHopPercent: string | null | undefined
  testId?: string
}) {
  const aside = impactAsideOutsideDetails({ routeSlippagePct, worstHopPercent })
  if (!aside) return null
  if (aside.kind === 'route') {
    return (
      <p className="mb-3 text-[11px] sm:text-xs" style={{ color: 'var(--ink-dim)' }} data-testid={testId}>
        Expected slippage {aside.percent}%.
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
