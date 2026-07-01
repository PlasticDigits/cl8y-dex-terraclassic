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
 * Pre-sign copy for **resting** limits: labeled action, pair, side, escrow amount, and chain before the wallet
 * opens ([#461](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/461) / SEC-I05), plus deviation,
 * maker placement fee, and network fee floor ([#157](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)).
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
  const effectivePct = effectiveFeeBps != null ? bpsToPercentLabel(effectiveFeeBps) : null

  return (
    <div
      className={`card-glass !p-2.5 space-y-2 ${textSize}`}
      data-testid={testId}
      role="region"
      aria-label="Limit order summary before signing"
    >
      <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
        Review these fields before your wallet opens. They must match what you intend to sign on{' '}
        <strong style={{ color: 'var(--ink-subtle)' }}>{chainFullLabel}</strong>.
      </p>
      <div className="space-y-1.5">
        <SummaryRow label="Action" value={actionLabel} testId={`${testId}-action`} />
        <SummaryRow label="Pair" value={pairLabel} testId={`${testId}-pair`} />
        <SummaryRow label="Side" value={sideLabel} testId={`${testId}-side`} />
        <SummaryRow label="Amount" value={escrowAmountLabel} testId={`${testId}-amount`} />
        <SummaryRow label="Chain" value={chainFullLabel} testId={`${testId}-chain`} />
      </div>
      <p className="text-[9px] leading-snug pt-1 border-t border-white/10" style={{ color: 'var(--ink-subtle)' }}>
        Resting limit — no immediate taker slippage, price impact, or min-received line like a market swap.
      </p>
      <ul className="list-disc pl-4 space-y-1" style={{ color: 'var(--ink-dim)' }}>
        <li>
          <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
            vs current reference:{' '}
          </span>
          {dev == null ? (
            <span className="opacity-80">—</span>
          ) : (
            <span className="font-mono tabular-nums" style={{ color: 'var(--ink)' }}>
              {dev > 0 ? '+' : ''}
              {dev.toFixed(1)}%
            </span>
          )}
          <span className="text-[9px] ml-1 opacity-90">(token1 per token0)</span>
        </li>
        <li data-testid={`${testId}-maker-fee`}>
          <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
            Maker fee (charged when placed):{' '}
          </span>
          {feeLoading ? (
            <span className="opacity-80">Loading…</span>
          ) : feeError || effectiveFeeBps == null || makerBps == null || makerPct == null ? (
            <span className="opacity-80">—</span>
          ) : (
            <span style={{ color: 'var(--ink)' }}>
              Small fee taken from your escrow at placement — about{' '}
              <strong className="font-mono tabular-nums">{makerPct}</strong> of escrow (
              <span className="font-mono tabular-nums">{makerBps}</span> bps; half of the{' '}
              <span className="font-mono tabular-nums">{effectivePct}</span> swap fee). The other half is charged to
              takers on each fill.
            </span>
          )}
        </li>
        <li>
          <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
            Est. network fee (min):{' '}
          </span>
          <span className="font-mono tabular-nums" style={{ color: 'var(--ink)' }}>
            ~{gasHuman} LUNC
          </span>
          <span className="text-[9px] ml-1 opacity-90">(allowance + place; wallet may vary)</span>
        </li>
      </ul>
      <p className="text-[9px] leading-snug pt-1 border-t border-white/10" style={{ color: 'var(--ink-subtle)' }}>
        <a className="underline hover:opacity-80" href={LIMIT_DOC} target="_blank" rel="noopener noreferrer">
          Learn more about limit order fees
        </a>
        .
      </p>
    </div>
  )
}
