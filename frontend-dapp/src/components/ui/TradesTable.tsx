import type { IndexerTrade, IndexerPair } from '@/types'
import { formatNum } from '@/utils/formatAmount'
import { getExplorerTxUrl } from '@/utils/terraExplorer'

export interface TradesTableProps {
  trades: IndexerTrade[]
  formatTimeFn: (iso: string) => string
  activePair?: IndexerPair
  ariaLabel?: string
}

export function TradesTable({ trades, formatTimeFn, activePair, ariaLabel }: TradesTableProps) {
  if (trades.length === 0) {
    return (
      <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>
        No trades
      </p>
    )
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs" aria-label={ariaLabel}>
        <thead>
          <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
              Time
            </th>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
              Direction
            </th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
              Offer
            </th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
              Return
            </th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
              Price
            </th>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
              Tx
            </th>
          </tr>
        </thead>
        <tbody>
          {trades.map((t) => {
            const isBuy = activePair && t.offer_asset === activePair.asset_0.symbol
            const hybrid =
              t.pool_return_amount != null || t.book_return_amount != null || t.limit_book_offer_consumed != null
            return (
              <tr key={t.id} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                <td className="py-1.5 px-2" style={{ color: 'var(--ink-subtle)' }}>
                  {formatTimeFn(t.block_timestamp)}
                </td>
                <td
                  className="py-1.5 px-2 font-medium"
                  style={{
                    color: activePair ? (isBuy ? 'var(--color-positive)' : 'var(--color-negative)') : 'var(--ink)',
                  }}
                >
                  {t.offer_asset} → {t.ask_asset}
                  {hybrid ? (
                    <span className="ml-1 font-normal opacity-70" title="Pool + limit book legs">
                      (hybrid)
                    </span>
                  ) : null}
                </td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>
                  {formatNum(t.offer_amount)}
                </td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>
                  {formatNum(t.return_amount)}
                </td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                  {formatNum(t.price, 6)}
                </td>
                <td className="py-1.5 px-2" style={{ color: 'var(--ink-dim)' }}>
                  {(() => {
                    const url = getExplorerTxUrl(t.tx_hash)
                    const label = `${t.tx_hash.slice(0, 8)}...`
                    return url ? (
                      <a href={url} target="_blank" rel="noopener noreferrer" className="underline hover:opacity-80">
                        {label}
                      </a>
                    ) : (
                      label
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
