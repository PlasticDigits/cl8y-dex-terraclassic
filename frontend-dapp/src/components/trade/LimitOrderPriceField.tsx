import { useId } from 'react'
import { formatNum } from '@/utils/formatAmount'
import {
  anchorUsdForLimitPrice,
  formatLimitPriceDeviationChipLabel,
  isLimitPriceDirectionInvalid,
  LIMIT_PRICE_DEVIATION_CHIP_PRESETS,
  limitPriceDeviationPercent,
  limitPriceFromRefDeviationChip,
  matchingLimitPriceDeviationChip,
  type LimitOrderPriceRefSource,
  parsePositivePriceHuman,
} from '@/utils/limitOrderPriceReference'

export function LimitOrderSideFlipButton({ onFlip, compact }: { onFlip: () => void; compact?: boolean }) {
  return (
    <div className={`flex justify-center ${compact ? '-my-1' : '-my-1.5'}`}>
      <button
        type="button"
        aria-label="Flip limit order side (bid / ask)"
        onClick={onFlip}
        className={`limit-side-flip-btn${compact ? ' limit-side-flip-btn-compact' : ''}`}
        data-testid="limit-order-side-flip"
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden>
          <path
            d="M8 1v14M8 1L4 5M8 1l4 4M8 15l-4-4M8 15l4-4"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </button>
    </div>
  )
}

export type LimitOrderPriceInputWithContextProps = {
  side: 'bid' | 'ask'
  price: string
  onPriceChange: (v: string) => void
  inputId: string
  /** Resolved token1/token0 reference (indexed tape and/or on-chain pool — see GitLab #166). */
  refToken1PerToken0: number | null
  /** Which source produced `refToken1PerToken0`, for display only. */
  refSource: LimitOrderPriceRefSource | null
  /** Same value the parent passes to `PriceChart` as `tapeLastPriceUsd` (newest trade `price` string). */
  tapeHeadlineUsd: string | null | undefined
  token0Label: string
  token1Label: string
  compact?: boolean
}

export function LimitOrderPriceInputWithContext({
  side,
  price,
  onPriceChange,
  inputId,
  refToken1PerToken0: ref,
  refSource: _refSource,
  tapeHeadlineUsd,
  token0Label,
  token1Label,
  compact,
}: LimitOrderPriceInputWithContextProps) {
  void _refSource
  const chipsUnavailableId = useId()
  const limit = parsePositivePriceHuman(price)
  const dev = ref != null && limit != null ? limitPriceDeviationPercent(limit, ref) : null
  const invalid = ref != null && limit != null && side ? isLimitPriceDirectionInvalid(side, limit, ref) : false
  const usd = ref != null && limit != null ? anchorUsdForLimitPrice(limit, ref, tapeHeadlineUsd) : null
  const activeChip = matchingLimitPriceDeviationChip(side, limit, ref)
  const chipsDisabled = ref == null || !(ref > 0)

  const extremeValidDeviation = !invalid && dev != null && Math.abs(dev) >= 50

  return (
    <div className="space-y-1.5">
      <div>
        <label className="label-glass" htmlFor={inputId}>
          When 1 {token0Label} is worth
        </label>
        <div className="flex items-center gap-2">
          <input
            id={inputId}
            className="input-glass w-full font-mono text-sm"
            value={price}
            onChange={(e) => onPriceChange(e.target.value)}
            aria-invalid={invalid}
            data-testid="limit-order-price-input"
          />
          <span
            className="shrink-0 text-xs font-semibold uppercase tracking-wide"
            style={{ color: 'var(--ink-subtle)' }}
          >
            {token1Label}
          </span>
        </div>
        <div
          className="mt-2 flex flex-wrap gap-2"
          role="group"
          aria-label="Limit price deviation from reference"
          data-testid="limit-order-price-deviation-chips"
        >
          {LIMIT_PRICE_DEVIATION_CHIP_PRESETS.map((pct) => {
            const isActive = activeChip === pct
            return (
              <button
                key={pct}
                type="button"
                className={`limit-pct-chip${isActive ? ' limit-pct-chip-active' : ''}`}
                disabled={chipsDisabled}
                aria-pressed={isActive}
                aria-describedby={chipsDisabled ? chipsUnavailableId : undefined}
                data-testid={`limit-order-price-chip-${pct}`}
                onClick={() => {
                  if (ref == null || !(ref > 0)) return
                  onPriceChange(limitPriceFromRefDeviationChip(side, ref, pct))
                }}
              >
                {formatLimitPriceDeviationChipLabel(side, pct)}
              </button>
            )
          })}
        </div>
        {chipsDisabled && (
          <span id={chipsUnavailableId} className="sr-only">
            Reference price unavailable — deviation chips disabled
          </span>
        )}
        <div
          className={`mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-0.5 ${compact ? 'text-[10px]' : 'text-xs'}`}
          style={{ color: 'var(--ink-dim)' }}
          data-testid="limit-order-price-context"
        >
          {ref != null && ref > 0 && (
            <span className="tabular-nums" style={{ color: 'var(--ink-subtle)' }}>
              Ref {formatNum(ref, 6)}
            </span>
          )}
          {dev != null && limit != null && (
            <span
              className="tabular-nums font-medium"
              style={{
                color: invalid
                  ? 'var(--color-negative, #ef4444)'
                  : extremeValidDeviation
                    ? 'var(--color-warning, #e8b84a)'
                    : 'var(--ink)',
              }}
            >
              {dev > 0 ? '+' : ''}
              {dev.toFixed(1)}%
            </span>
          )}
          {invalid && (
            <span className="font-medium" style={{ color: 'var(--color-negative, #ef4444)' }} role="alert">
              Invalid {side === 'bid' ? 'buy' : 'sell'}
            </span>
          )}
          {usd != null && Number.isFinite(usd) && (
            <span className="tabular-nums opacity-80">≈ ${formatNum(usd, 4)}</span>
          )}
        </div>
      </div>
    </div>
  )
}
