import {
  formatTerraClassicFeeLunc,
  type TerraClassicFeeEstimate,
} from '@/services/terraclassic/terraClassicFeeEstimate'

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
  'data-testid': testId = 'terra-classic-tx-fee-hint',
}: TerraClassicTxFeeHintProps) {
  const textSize = compact ? 'text-[10px]' : 'text-xs'
  const lunc = formatTerraClassicFeeLunc(estimate.feeUluna)

  return (
    <p
      className={`${textSize} tabular-nums ${className}`.trim()}
      style={{ color: 'var(--ink-dim)' }}
      data-testid={testId}
    >
      <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
        {label}:{' '}
      </span>
      <span className="font-mono" style={{ color: 'var(--ink)' }}>
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
