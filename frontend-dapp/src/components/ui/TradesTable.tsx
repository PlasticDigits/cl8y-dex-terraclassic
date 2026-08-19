import type { IndexerTrade, IndexerPair } from '@/types'
import { getExplorerTxUrl, shortenTxHashForDisplay } from '@/utils/terraExplorer'
import {
  formatTapeAmount,
  formatTapePrice,
  resolveAskDecimals,
  resolveOfferDecimals,
  tapePriceTooltip,
  tapeRowIsBuy,
} from '@/utils/tradeTapeDisplay'

export interface TradesTableProps {
  trades: IndexerTrade[]
  formatTimeFn: (iso: string) => string
  activePair?: IndexerPair
  /** Display invert from `/trade` / `/charts` (#524). Trader/Portfolio leave false. */
  inverted?: boolean
  ariaLabel?: string
}

export function TradesTable({ trades, formatTimeFn, activePair, inverted = false, ariaLabel }: TradesTableProps) {
  if (trades.length === 0) {
    return (
      <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>
        No trades
      </p>
    )
  }

  const priceTitle = tapePriceTooltip(activePair, inverted)

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs" aria-label={ariaLabel}>
        <thead>
          <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
              Time
            </th>
            <th
              scope="col"
              className="text-left py-2 px-2 font-medium uppercase tracking-wider"
              title="Assets swapped in this fill (pay → receive)"
            >
              Pair
            </th>
            <th
              scope="col"
              className="text-right py-2 px-2 font-medium uppercase tracking-wider"
              title="Amount of the asset paid in (offer side)"
            >
              Amount in
            </th>
            <th
              scope="col"
              className="text-right py-2 px-2 font-medium uppercase tracking-wider"
              title="Amount of the asset received out (ask side)"
            >
              Amount out
            </th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider" title={priceTitle}>
              Price
            </th>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
              Tx
            </th>
          </tr>
        </thead>
        <tbody>
          {trades.map((t) => {
            const isBuy = tapeRowIsBuy(t, activePair, inverted)
            const hybrid =
              t.pool_return_amount != null || t.book_return_amount != null || t.limit_book_offer_consumed != null
            const offerDec = resolveOfferDecimals(t, activePair)
            const askDec = resolveAskDecimals(t, activePair)
            const rowColor = isBuy == null ? 'var(--ink)' : isBuy ? 'var(--color-positive)' : 'var(--color-negative)'
            return (
              <tr key={t.id} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                <td className="py-1.5 px-2" style={{ color: 'var(--ink-subtle)' }}>
                  {formatTimeFn(t.block_timestamp)}
                </td>
                <td className="py-1.5 px-2 font-medium" style={{ color: rowColor }}>
                  {t.offer_asset} → {t.ask_asset}
                  {hybrid ? (
                    <span
                      className="ml-1 inline-flex items-center rounded border border-white/20 px-1 py-px text-[10px] font-normal uppercase tracking-wide text-white/70"
                      title="Executed via hybrid AMM + limit order routing. Fee lines may split across pool hooks and limit_order_fill events — see docs/integrators.md."
                    >
                      hybrid
                    </span>
                  ) : null}
                </td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>
                  {formatTapeAmount(t.offer_amount, offerDec, t.offer_asset)}
                </td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>
                  {formatTapeAmount(t.return_amount, askDec, t.ask_asset)}
                </td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                  {formatTapePrice(t.price, inverted)}
                </td>
                <td className="py-1.5 px-2" style={{ color: 'var(--ink-dim)' }}>
                  {(() => {
                    const url = getExplorerTxUrl(t.tx_hash)
                    const label = shortenTxHashForDisplay(t.tx_hash)
                    return url ? (
                      <a
                        href={url}
                        target="_blank"
                        rel="noopener noreferrer"
                        title={t.tx_hash}
                        className="underline hover:opacity-80"
                      >
                        {label}
                      </a>
                    ) : (
                      <span title={t.tx_hash}>{label}</span>
                    )
                  })()}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
