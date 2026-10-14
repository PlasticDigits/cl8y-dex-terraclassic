import { fromRawAmount } from '@/utils/formatAmount'
import type { RouteOrderCosts } from '@/utils/routeOrderMetrics'

/** Common final-output units; network gas is disclosed separately in LUNC. */
export function RouteOrderCostDetails({
  costs,
  symbol,
  decimals,
}: {
  costs: RouteOrderCosts | null | undefined
  symbol: string
  decimals: number | null | undefined
}) {
  if (!costs || decimals == null) return <p className="text-xs">Cost breakdown unavailable.</p>
  return (
    <dl className="text-xs space-y-1" data-testid="route-order-cost-details">
      {(
        [
          ['Trading fees', costs.tradingFeeRaw],
          ['Tax and hook costs', costs.taxHookRaw],
          ['Wrapping costs', costs.wrapperRaw],
          ['Reference price difference', costs.referenceDivergenceRaw],
        ] as const
      ).map(([label, raw]) => (
        <div key={label} className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.6fr)] items-baseline gap-x-2">
          <dt>{label}</dt>
          <dd className="text-right">
            {raw.startsWith('-') ? '-' : ''}
            {decimals === 0 ? raw.replace(/^-/, '') : fromRawAmount(raw.replace(/^-/, ''), decimals)} {symbol}
            <span className="font-mono block text-[10px]">{raw} raw</span>
          </dd>
        </div>
      ))}
      <p>Fee basis: fee-free route spot output. Negative slippage estimates a price improvement.</p>
    </dl>
  )
}
