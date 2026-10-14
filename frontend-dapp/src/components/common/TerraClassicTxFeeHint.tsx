import {
  formatTerraClassicFeeLunc,
  type TerraClassicFeeEstimate,
} from '@/services/terraclassic/terraClassicFeeEstimate'
import { useNumberDisplayMode } from '@/utils/numberDisplayPreference'

export type TerraClassicTxFeeHintProps = {
  estimate: TerraClassicFeeEstimate
  /** Optional prefix, e.g. "Network fee (est.)" */
  label?: string
  compact?: boolean
  /**
   * Show `gas × price uluna` parenthetical. Default **off** — too technical for
   * retail Swap chrome ([GitLab #587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587) / #489).
   */
  showInternals?: boolean
  className?: string
  aligned?: boolean
  'data-testid'?: string
}

/**
 * Shared network-fee hint for Terra Classic txs (gas limit × Classic min gas price, not Terra v2 / LCD sim).
 * Retail default is `~X LUNC` only; internals belong in Trade details.
 */
export function TerraClassicTxFeeHint({
  estimate,
  label = 'Network fee (est.)',
  compact,
  showInternals = false,
  className = '',
  aligned = false,
  'data-testid': testId = 'terra-classic-tx-fee-hint',
}: TerraClassicTxFeeHintProps) {
  useNumberDisplayMode()
  const textSize = compact ? 'text-[10px]' : 'text-xs'
  const lunc = formatTerraClassicFeeLunc(estimate.feeUluna)

  if (estimate.coverageError)
    return (
      <p className={className} data-testid={testId}>
        Network fee unavailable
      </p>
    )

  return (
    <p
      className={`${textSize} tabular-nums ${aligned ? 'grid grid-cols-[minmax(0,1fr)_minmax(0,1.6fr)] items-baseline gap-x-3' : ''} ${className}`.trim()}
      style={{ color: 'var(--ink-dim)' }}
      data-testid={testId}
    >
      <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
        {label}
        {aligned ? '' : ': '}
      </span>
      <span className={`font-mono ${aligned ? 'text-right' : ''}`} style={{ color: 'var(--ink)' }}>
        ~{lunc} LUNC
      </span>
      {showInternals ? (
        <span className="opacity-80 ml-1">
          ({estimate.gasLimit.toLocaleString()} gas × {estimate.gasPriceUluna} uluna)
        </span>
      ) : null}
    </p>
  )
}
