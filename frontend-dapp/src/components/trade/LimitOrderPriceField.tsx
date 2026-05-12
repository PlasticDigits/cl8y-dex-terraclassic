import { useId } from 'react'
import type { IndexerPair, IndexerTrade } from '@/types'
import { formatNum } from '@/utils/formatAmount'
import {
  anchorUsdForLimitPrice,
  isLimitPriceDirectionInvalid,
  limitPriceDeviationPercent,
  parsePositivePriceHuman,
  tradeToToken1PerToken0Human,
} from '@/utils/limitOrderPriceReference'

const LIMIT_ORDER_TOOLTIP =
  'A limit order lets you set the price you are willing to buy or sell at. Buy limits should be set below the current reference; sell limits above. Prices use token1 per token0 (pair ordering).'

export function LimitOrderPlaceLimitHeading({ compact }: { compact?: boolean }) {
  const tipId = useId()
  return (
    <div className="flex flex-wrap items-center gap-2">
      <h3
        className={`font-semibold uppercase tracking-wide ${compact ? 'text-xs' : 'text-sm'}`}
        style={{ color: 'var(--ink)' }}
      >
        Place limit
      </h3>
      <span
        className="inline-flex h-5 w-5 cursor-help select-none items-center justify-center rounded-full border border-white/25 text-[10px] font-bold leading-none"
        style={{ color: 'var(--ink-dim)' }}
        tabIndex={0}
        role="img"
        aria-label="Limit order help"
        aria-describedby={tipId}
        title={LIMIT_ORDER_TOOLTIP}
      >
        i
      </span>
      <span id={tipId} className="sr-only">
        {LIMIT_ORDER_TOOLTIP}
      </span>
    </div>
  )
}

export type LimitOrderPriceInputWithContextProps = {
  side: 'bid' | 'ask'
  price: string
  onPriceChange: (v: string) => void
  inputId: string
  activePair: IndexerPair | null | undefined
  latestTrade: IndexerTrade | null | undefined
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
  activePair,
  latestTrade,
  tapeHeadlineUsd,
  token0Label,
  token1Label,
  compact,
}: LimitOrderPriceInputWithContextProps) {
  const ref = activePair && latestTrade ? tradeToToken1PerToken0Human(latestTrade, activePair) : null
  const limit = parsePositivePriceHuman(price)
  const dev = ref != null && limit != null ? limitPriceDeviationPercent(limit, ref) : null
  const invalid = ref != null && limit != null && side ? isLimitPriceDirectionInvalid(side, limit, ref) : false
  const usd = ref != null && limit != null ? anchorUsdForLimitPrice(limit, ref, tapeHeadlineUsd) : null

  const extremeValidDeviation = !invalid && dev != null && Math.abs(dev) >= 50

  const refLine =
    ref != null && ref > 0 ? (
      <span className="tabular-nums">
        {formatNum(ref, 6)} {token1Label}/{token0Label}
      </span>
    ) : (
      <span className="opacity-80">—</span>
    )

  const devLine =
    dev == null || limit == null ? (
      <span className="opacity-80">—</span>
    ) : (
      <span
        className="tabular-nums font-medium"
        style={{
          color: invalid ? 'var(--color-negative, #ef4444)' : extremeValidDeviation ? '#f59e0b' : 'var(--ink)',
        }}
      >
        {dev > 0 ? '+' : ''}
        {dev.toFixed(1)}%
      </span>
    )

  const usdLine =
    usd != null && Number.isFinite(usd) ? (
      <span className="tabular-nums">≈ ${formatNum(usd, 4)}</span>
    ) : (
      <span className="opacity-80">—</span>
    )

  return (
    <div className="space-y-1.5">
      <div>
        <label className="label-neo" htmlFor={inputId}>
          Price (token1 per token0)
        </label>
        <input
          id={inputId}
          className="input-neo w-full font-mono text-sm"
          value={price}
          onChange={(e) => onPriceChange(e.target.value)}
          aria-invalid={invalid}
          data-testid="limit-order-price-input"
        />
        <div
          className={`mt-1.5 space-y-0.5 ${compact ? 'text-[10px]' : 'text-xs'}`}
          style={{ color: 'var(--ink-dim)' }}
          data-testid="limit-order-price-context"
        >
          <p>
            <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
              Current (last trade):{' '}
            </span>
            {refLine}
          </p>
          <p>
            <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
              vs reference:{' '}
            </span>
            {devLine}
            {invalid && (
              <span className="ml-1.5 font-medium" style={{ color: 'var(--color-negative, #ef4444)' }}>
                (invalid for {side === 'bid' ? 'buy' : 'sell'} limit)
              </span>
            )}
          </p>
          <p>
            <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
              Headline-scaled USD:{' '}
            </span>
            {usdLine}
          </p>
        </div>
      </div>
    </div>
  )
}
