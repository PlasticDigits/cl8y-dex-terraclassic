import { getNetworkBadgeCopy } from '@/utils/networkDisplay'

export type PoolPreSubmitSummaryProps = {
  /** Wallet action shown before extension signing, e.g. "Provide Liquidity" / "Withdraw Liquidity". */
  actionLabel: string
  /** Pair label, e.g. "EMBER / CORAL". */
  pairLabel: string
  /** Ordered amount lines, e.g. ["12.5 EMBER", "8.0 CORAL"] (provide) or ["3.0 LP"] (withdraw). */
  amountLines: string[]
  /** Active Terra Classic network label (defaults to env badge copy). */
  chainFullLabel?: string
  'data-testid'?: string
}

function SummaryRow({ label, value, testId }: { label: string; value: string; testId?: string }) {
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
 * Compact pre-sign summary for pool provide / withdraw: labeled action, pair, amounts, and chain so a
 * phishing page cannot substitute pair/chain without failing tests — the same SEC-D11 anti-phishing
 * anchor swaps already have ([#462](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/462) / SEC-I05).
 *
 * Pool forms already show token inputs above the submit button; this card repeats only the security
 * anchors (action, pair, consolidated amounts, chain) without duplicating the swap-style intro copy.
 */
export function PoolPreSubmitSummary({
  actionLabel,
  pairLabel,
  amountLines,
  chainFullLabel = getNetworkBadgeCopy().fullLabel,
  'data-testid': testId = 'pool-pre-submit-summary',
}: PoolPreSubmitSummaryProps) {
  const amountDisplay = amountLines.join(' + ')

  return (
    <div
      className="card-glass !p-2.5 space-y-1.5 text-[11px] sm:text-xs mb-3"
      data-testid={testId}
      role="region"
      aria-label="Liquidity action summary before signing"
    >
      <div className="space-y-1.5">
        <SummaryRow label="Action" value={actionLabel} testId={`${testId}-action`} />
        <SummaryRow label="Pair" value={pairLabel} testId={`${testId}-pair`} />
        <SummaryRow label="Amount" value={amountDisplay} testId={`${testId}-amount`} />
        <SummaryRow label="Chain" value={chainFullLabel} testId={`${testId}-chain`} />
      </div>
    </div>
  )
}
