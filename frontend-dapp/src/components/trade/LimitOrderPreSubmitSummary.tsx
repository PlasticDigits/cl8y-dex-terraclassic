import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { formatTokenAmount } from '@/utils/formatAmount'
import { bpsToPercentLabel } from '@/utils/limitOrderFeeSummary'
import { limitPriceDeviationPercent, parsePositivePriceHuman } from '@/utils/limitOrderPriceReference'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'

const LIMIT_DOC = `${DOCS_GITLAB_BASE}/limit-orders.md`

export type LimitOrderPreSubmitSummaryProps = {
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

/**
 * Pre-sign copy for **resting** limits: no immediate execution, no taker-style slippage lines;
 * instead deviation vs reference + maker placement fee + network fee floor ([GitLab #157](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)).
 */
export function LimitOrderPreSubmitSummary({
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
      <p className="leading-snug" style={{ color: 'var(--ink-dim)' }}>
        A <strong style={{ color: 'var(--ink-subtle)' }}>limit order</strong> does not trade immediately. It rests on
        the book until other traders fill it over time, so there is{' '}
        <strong style={{ color: 'var(--ink)' }}>no taker slippage</strong>,{' '}
        <strong style={{ color: 'var(--ink)' }}>no</strong> pool{' '}
        <strong style={{ color: 'var(--ink)' }}>price impact</strong>, and{' '}
        <strong style={{ color: 'var(--ink)' }}>no “min received”</strong> line like a market swap — those apply when
        you take liquidity now (Market tab / hybrid swap).
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
        <li data-testid={`${testId}-chain`}>
          <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
            Chain:{' '}
          </span>
          <span style={{ color: 'var(--ink)' }}>{chainFullLabel}</span>
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
