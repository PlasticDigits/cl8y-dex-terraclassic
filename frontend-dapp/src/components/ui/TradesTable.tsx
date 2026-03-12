import type { IndexerTrade, IndexerPair } from '@/types'
import { formatNum } from '@/utils/formatAmount'

export interface TradesTableProps {
  trades: IndexerTrade[]
  formatTimeFn: (iso: string) => string
  activePair?: IndexerPair
  ariaLabel?: string
}

export function TradesTable({ trades, formatTimeFn, activePair, ariaLabel }: TradesTableProps) {
  if (trades.length === 0) {
    return <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>No trades</p>
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs" aria-label={ariaLabel}>
        <thead>
          <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">Time</th>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">Direction</th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">Offer</th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">Return</th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">Price</th>
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">Tx</th>
          </tr>
        </thead>
        <tbody>
          {trades.map((t) => {
            const isBuy = activePair && t.offer_asset === activePair.asset_0.symbol
            return (
              <tr key={t.id} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                <td className="py-1.5 px-2" style={{ color: 'var(--ink-subtle)' }}>{formatTimeFn(t.block_timestamp)}</td>
                <td
                  className="py-1.5 px-2 font-medium"
                  style={{ color: activePair ? (isBuy ? 'var(--color-positive)' : 'var(--color-negative)') : 'var(--ink)' }}
                >
                  {t.offer_asset} → {t.ask_asset}
                </td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>{formatNum(t.offer_amount)}</td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>{formatNum(t.return_amount)}</td>
                <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>{formatNum(t.price, 6)}</td>
                <td className="py-1.5 px-2" style={{ color: 'var(--ink-dim)' }}>{t.tx_hash.slice(0, 8)}...</td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
