import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { formatTokenAmount } from '@/utils/formatAmount'
import { bpsToPercentLabel } from '@/utils/limitOrderFeeSummary'
import { limitPriceDeviationPercent, parsePositivePriceHuman } from '@/utils/limitOrderPriceReference'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'

const LIMIT_DOC = `${DOCS_GITLAB_BASE}/limit-orders.md`

export type LimitOrderPreSubmitSummaryProps = {
  /** Wallet action shown before extension signing (default: Place Limit Order). */
  actionLabel?: string
  /** Pair label, e.g. "EMBER / CORAL". */
  pairLabel: string
  /** Side label, e.g. "Buy EMBER" / "Sell EMBER". */
  sideLabel: string
  /** Escrow amount line, e.g. "12.5 CORAL". */
  escrowAmountLabel: string
  /** Minimum uluna for increase_allowance + place (sequence estimate). */
  placeSequenceMinUluna: bigint
  /** Resolved reference token1/token0 (tape or pool). */
  refToken1PerToken0: number | null
  typedPrice: string
  /** Effective swap fee bps after discount; null while loading / unavailable. */
  effectiveFeeBps: number | null
  /** `floor(effective/2)` — maker placement leg; null when `effectiveFeeBps` is null. */
  makerPlacementFeeBps: number | null
  feeLoading: boolean
  feeError: boolean
  compact?: boolean
  /** Active Terra Classic network label shown as an anti-phishing chain anchor (SEC-I05 / #461). */
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
 * Compact pre-sign summary for resting limits (#461 / SEC-I05 chain anchor; #488 copy minimization).
 * Keeps labeled action/pair/side/amount/chain — strips instructional paragraphs.
 */
export function LimitOrderPreSubmitSummary({
  actionLabel = 'Place Limit Order',
  pairLabel,
  sideLabel,
  escrowAmountLabel,
  placeSequenceMinUluna,
  refToken1PerToken0,
  typedPrice,
  effectiveFeeBps,
  makerPlacementFeeBps: makerBps,
  feeLoading,
  feeError,
  compact,
  chainFullLabel = getNetworkBadgeCopy().fullLabel,
  'data-testid': testId = 'limit-order-pre-submit-summary',
}: LimitOrderPreSubmitSummaryProps) {
  const textSize = compact ? 'text-[10px]' : 'text-xs'
  const limit = parsePositivePriceHuman(typedPrice)
  const dev = refToken1PerToken0 != null && limit != null ? limitPriceDeviationPercent(limit, refToken1PerToken0) : null

  const gasHuman = formatTokenAmount(placeSequenceMinUluna.toString(), 6, 4)
  const makerPct = makerBps != null ? bpsToPercentLabel(makerBps) : null

  return (
    <div
      className={`card-glass !p-2.5 space-y-2 ${textSize}`}
      data-testid={testId}
      role="region"
      aria-label="Limit order summary before signing"
    >
      <div className="space-y-1.5">
        <SummaryRow label="Action" value={actionLabel} testId={`${testId}-action`} />
        <SummaryRow label="Pair" value={pairLabel} testId={`${testId}-pair`} />
        <SummaryRow label="Side" value={sideLabel} testId={`${testId}-side`} />
        <SummaryRow label="Pay" value={escrowAmountLabel} testId={`${testId}-amount`} />
        <SummaryRow label="Chain" value={chainFullLabel} testId={`${testId}-chain`} />
        <SummaryRow
          label="vs ref"
          value={dev == null ? '—' : `${dev > 0 ? '+' : ''}${dev.toFixed(1)}%`}
          testId={`${testId}-vs-ref`}
        />
        <div data-testid={`${testId}-maker-fee`}>
          <SummaryRow
            label="Maker fee"
            value={
              feeLoading
                ? '…'
                : feeError || effectiveFeeBps == null || makerBps == null || makerPct == null
                  ? '—'
                  : makerPct
            }
            testId={`${testId}-maker-fee-row`}
          />
        </div>
        <SummaryRow label="Gas (min)" value={`~${gasHuman} LUNC`} testId={`${testId}-gas`} />
      </div>
      <p className="text-[9px] leading-snug pt-1 border-t border-white/10" style={{ color: 'var(--ink-subtle)' }}>
        <a className="underline hover:opacity-80" href={LIMIT_DOC} target="_blank" rel="noopener noreferrer">
          Docs
        </a>
      </p>
    </div>
  )
}
