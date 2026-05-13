import * as React from 'react'
import { useQuery } from '@tanstack/react-query'
import type { IndexerLimitCancellation, IndexerLimitFill, IndexerTrade } from '@/types'
import {
  downloadTextAsFile,
  fetchTraderHistoryCsv,
  getTraderLimitCancellations,
  getTraderLimitFills,
  getTraderTrades,
  type TraderHistoryCsvResource,
} from '@/services/indexer/client'
import { formatDateTime } from '@/utils/formatDate'
import { getExplorerTxUrl, shortenTxHashForDisplay } from '@/utils/terraExplorer'
import { RetryError, Skeleton } from '@/components/ui'

export type WalletIndexerHistorySection = 'swaps' | 'fills' | 'cancels'

export interface WalletIndexerHistoryPanelProps {
  walletAddress: string
  pairAddress: string
  /** Defaults to all three (limits page). Trade page typically passes `['swaps']` only. */
  sections?: WalletIndexerHistorySection[]
}

function txCell(txHash: string) {
  const url = getExplorerTxUrl(txHash)
  const label = shortenTxHashForDisplay(txHash)
  if (url) {
    return (
      <a
        className="underline font-mono text-[11px] hover:opacity-80"
        href={url}
        target="_blank"
        rel="noopener noreferrer"
      >
        {label}
      </a>
    )
  }
  return (
    <span className="font-mono text-[11px]" title={txHash}>
      {label}
    </span>
  )
}

function swapFeeLabel(t: IndexerTrade): string {
  if (t.commission_amount != null && t.commission_amount !== '') return t.commission_amount
  if (t.effective_fee_bps != null) return `${t.effective_fee_bps} bps`
  return '—'
}

async function downloadCsv(resource: TraderHistoryCsvResource, address: string, pairAddress: string, slug: string) {
  const csv = await fetchTraderHistoryCsv(resource, address, { pair: pairAddress, limit: 500 })
  const base = resource === 'trades' ? 'swaps' : resource === 'limit-fills' ? 'limit-fills' : 'limit-cancellations'
  downloadTextAsFile(`${base}-${slug}.csv`, csv)
}

function HistoryBlock<T>({
  title,
  query,
  onDownloadCsv,
  emptyLabel,
  children,
}: {
  title: string
  query: { isLoading: boolean; isError: boolean; data: T[] | undefined; refetch: () => unknown }
  onDownloadCsv: () => Promise<void>
  emptyLabel: string
  children: (rows: T[]) => React.ReactNode
}) {
  const [pending, setPending] = React.useState(false)
  const onCsv = async () => {
    setPending(true)
    try {
      await onDownloadCsv()
    } finally {
      setPending(false)
    }
  }

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
          {title}
        </h3>
        <button
          type="button"
          className="text-xs py-1 px-2 rounded border border-white/15 hover:bg-white/5 transition-colors"
          disabled={pending}
          onClick={() => void onCsv()}
        >
          {pending ? '…' : 'Download CSV'}
        </button>
      </div>
      {query.isLoading && <Skeleton height="4rem" />}
      {query.isError && <RetryError message="Failed to load history" onRetry={() => void query.refetch()} />}
      {query.data && query.data.length === 0 && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
          {emptyLabel}
        </p>
      )}
      {query.data && query.data.length > 0 && children(query.data)}
    </div>
  )
}

export function WalletIndexerHistoryPanel({
  walletAddress,
  pairAddress,
  sections = ['swaps', 'fills', 'cancels'],
}: WalletIndexerHistoryPanelProps) {
  const enabled = walletAddress.length > 0 && pairAddress.startsWith('terra1')
  const slug = walletAddress.replace(/[^a-zA-Z0-9]/g, '').slice(0, 20) || 'wallet'

  const showSwaps = sections.includes('swaps')
  const showFills = sections.includes('fills')
  const showCancels = sections.includes('cancels')

  const swapsQuery = useQuery({
    queryKey: ['wallet-indexer-history', 'swaps', walletAddress, pairAddress],
    queryFn: () => getTraderTrades(walletAddress, { pair: pairAddress, limit: 50 }),
    enabled: enabled && showSwaps,
    staleTime: 15_000,
  })

  const fillsQuery = useQuery({
    queryKey: ['wallet-indexer-history', 'fills', walletAddress, pairAddress],
    queryFn: () => getTraderLimitFills(walletAddress, { pair: pairAddress, limit: 50 }),
    enabled: enabled && showFills,
    staleTime: 15_000,
  })

  const cancelsQuery = useQuery({
    queryKey: ['wallet-indexer-history', 'cancels', walletAddress, pairAddress],
    queryFn: () => getTraderLimitCancellations(walletAddress, { pair: pairAddress, limit: 50 }),
    enabled: enabled && showCancels,
    staleTime: 15_000,
  })

  if (!enabled) return null

  return (
    <div className="card-neo !p-4 space-y-6" data-testid="wallet-indexer-history">
      <div>
        <h2 className="text-sm font-semibold uppercase tracking-wide">Your history (this pair)</h2>
        <p className="text-xs mt-1" style={{ color: 'var(--ink-dim)' }}>
          Indexed swaps, limit fills, and cancellations for your wallet on the selected pair. CSV export uses the same
          indexer filters (GitLab #163).
        </p>
      </div>

      {showSwaps && (
        <HistoryBlock
          title="Swaps (AMM)"
          query={swapsQuery}
          onDownloadCsv={() => downloadCsv('trades', walletAddress, pairAddress, slug)}
          emptyLabel="No indexed swaps for this wallet on this pair."
        >
          {(rows: IndexerTrade[]) => (
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="py-1.5 pr-2 font-medium">Time</th>
                  <th className="py-1.5 pr-2 font-medium">Side</th>
                  <th className="py-1.5 pr-2 font-medium">Price</th>
                  <th className="py-1.5 pr-2 font-medium">Fee</th>
                  <th className="py-1.5 font-medium">Tx</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((t) => (
                  <tr key={t.id} className="border-b border-white/5 font-mono">
                    <td className="py-1.5 pr-2 whitespace-nowrap">{formatDateTime(t.block_timestamp)}</td>
                    <td className="py-1.5 pr-2">
                      {t.offer_asset} → {t.ask_asset}
                    </td>
                    <td className="py-1.5 pr-2">{t.price}</td>
                    <td className="py-1.5 pr-2">{swapFeeLabel(t)}</td>
                    <td className="py-1.5">{txCell(t.tx_hash)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </HistoryBlock>
      )}

      {showFills && (
        <HistoryBlock
          title="Limit fills (maker)"
          query={fillsQuery}
          onDownloadCsv={() => downloadCsv('limit-fills', walletAddress, pairAddress, slug)}
          emptyLabel="No indexed limit fills for this wallet on this pair."
        >
          {(rows: IndexerLimitFill[]) => (
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="py-1.5 pr-2 font-medium">Time</th>
                  <th className="py-1.5 pr-2 font-medium">Order</th>
                  <th className="py-1.5 pr-2 font-medium">Side</th>
                  <th className="py-1.5 pr-2 font-medium">Price</th>
                  <th className="py-1.5 pr-2 font-medium">Commission</th>
                  <th className="py-1.5 font-medium">Tx</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((r) => (
                  <tr key={r.id} className="border-b border-white/5 font-mono">
                    <td className="py-1.5 pr-2 whitespace-nowrap">{formatDateTime(r.block_timestamp)}</td>
                    <td className="py-1.5 pr-2">#{r.order_id}</td>
                    <td className="py-1.5 pr-2">{r.side}</td>
                    <td className="py-1.5 pr-2">{r.price}</td>
                    <td className="py-1.5 pr-2">{r.commission_amount}</td>
                    <td className="py-1.5">{txCell(r.tx_hash)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </HistoryBlock>
      )}

      {showCancels && (
        <HistoryBlock
          title="Limit cancellations"
          query={cancelsQuery}
          onDownloadCsv={() => downloadCsv('limit-cancellations', walletAddress, pairAddress, slug)}
          emptyLabel="No indexed cancellations for this wallet on this pair."
        >
          {(rows: IndexerLimitCancellation[]) => (
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="py-1.5 pr-2 font-medium">Time</th>
                  <th className="py-1.5 pr-2 font-medium">Order</th>
                  <th className="py-1.5 font-medium">Tx</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((r) => (
                  <tr key={r.id} className="border-b border-white/5 font-mono">
                    <td className="py-1.5 pr-2 whitespace-nowrap">{formatDateTime(r.block_timestamp)}</td>
                    <td className="py-1.5 pr-2">#{r.order_id}</td>
                    <td className="py-1.5">{txCell(r.tx_hash)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </HistoryBlock>
      )}
    </div>
  )
}
