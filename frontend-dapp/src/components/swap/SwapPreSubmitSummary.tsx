import { getNetworkBadgeCopy } from '@/utils/networkDisplay'

export type SwapPreSubmitSummaryProps = {
  /** Wallet action shown before extension signing (default: Swap). */
  actionLabel?: string
  offerSymbol: string
  receiveSymbol: string
  offerAmountHuman: string
  receiveAmountHuman: string
  /** Slippage tolerance passed on-chain as max spread (percent). */
  maxSpreadPercent: number
  /** Minimum receive after slippage floor; null while quote is incomplete. */
  minReceiveHuman: string | null
  /** Active Terra Classic network label (defaults to env badge copy). */
  chainFullLabel?: string
  'data-testid'?: string
}

function SummaryRow({ label, value, testId }: { label: string; value: string; testId: string }) {
  return (
    <div className="flex flex-col gap-0.5 sm:flex-row sm:items-start sm:justify-between" data-testid={testId}>
      <span className="uppercase text-[10px] tracking-wide font-medium shrink-0" style={{ color: 'var(--ink-subtle)' }}>
        {label}
      </span>
      <span className="font-mono text-[10px] sm:text-right break-words min-w-0" style={{ color: 'var(--ink)' }}>
        {value}
      </span>
    </div>
  )
}

/**
 * Pre-sign summary for taker swaps: labeled action, pair, amounts, max spread, min return, and chain
 * so wallet phishing cannot substitute pair/chain without failing tests ([#409](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/409) / SEC-D11).
 */
export function SwapPreSubmitSummary({
  actionLabel = 'Swap',
  offerSymbol,
  receiveSymbol,
  offerAmountHuman,
  receiveAmountHuman,
  maxSpreadPercent,
  minReceiveHuman,
  chainFullLabel = getNetworkBadgeCopy().fullLabel,
  'data-testid': testId = 'swap-pre-submit-summary',
}: SwapPreSubmitSummaryProps) {
  const pairLabel = `${offerSymbol} → ${receiveSymbol}`

  return (
    <div
      className="card-glass !p-2.5 space-y-2 text-[11px] sm:text-xs mb-3"
      data-testid={testId}
      role="region"
      aria-label="Swap summary before signing"
    >
      <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
        Review these fields before your wallet opens. They must match what you intend to sign on{' '}
        <strong style={{ color: 'var(--ink-subtle)' }}>{chainFullLabel}</strong>.
      </p>
      <div className="space-y-1.5">
        <SummaryRow label="Action" value={actionLabel} testId="swap-confirm-action" />
        <SummaryRow label="Pair" value={pairLabel} testId="swap-confirm-pair" />
        <SummaryRow label="You pay" value={`${offerAmountHuman} ${offerSymbol}`} testId="swap-confirm-offer" />
        <SummaryRow
          label="You receive (est.)"
          value={`${receiveAmountHuman} ${receiveSymbol}`}
          testId="swap-confirm-receive"
        />
        <SummaryRow label="Max spread" value={`${maxSpreadPercent}%`} testId="swap-confirm-max-spread" />
        <SummaryRow
          label="Min return"
          value={minReceiveHuman != null ? `${minReceiveHuman} ${receiveSymbol}` : '—'}
          testId="swap-confirm-min-return"
        />
        <SummaryRow label="Chain" value={chainFullLabel} testId="swap-confirm-chain" />
      </div>
    </div>
  )
}
