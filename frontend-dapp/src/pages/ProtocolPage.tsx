import { useState, useDeferredValue } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getHookEvents, getOraclePrice, getOracleHistory } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { StatBox, RetryError, Skeleton } from '@/components/ui'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, PROTOCOL_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import { detectMarketDataOutage } from '@/utils/marketDataOutage'
import { formatNum } from '@/utils/formatAmount'
import { formatDateTime } from '@/utils/formatDate'
import { shortenAddress } from '@/utils/tokenDisplay'
import { AddressRow } from '@/components/ui/AddressRow'
import {
  FACTORY_CONTRACT_ADDRESS,
  ROUTER_CONTRACT_ADDRESS,
  TERRA_LCD_URL,
  TERRA_RPC_URL,
  DEFAULT_NETWORK,
} from '@/utils/constants'

function formatHookAmount(amount: string | number | null | undefined): string {
  if (amount == null || amount === '') return '—'
  return typeof amount === 'number' ? String(amount) : amount
}

export default function ProtocolPage() {
  const [hookFilter, setHookFilter] = useState('')
  const deferredHookFilter = useDeferredValue(hookFilter.trim())

  const priceQuery = useQuery({
    queryKey: ['indexer-oracle-price'],
    queryFn: () => getOraclePrice(),
    refetchInterval: 60_000,
    retry: false,
  })

  const historyQuery = useQuery({
    queryKey: ['indexer-oracle-history'],
    queryFn: () => getOracleHistory({ limit: 48 }),
    staleTime: 60_000,
    refetchInterval: 120_000,
    retry: false,
  })

  const hooksQuery = useQuery({
    queryKey: ['indexer-hooks', deferredHookFilter || 'all'],
    queryFn: () =>
      getHookEvents({
        hook_address: deferredHookFilter || undefined,
        limit: 50,
      }),
    refetchInterval: 30_000,
    retry: false,
  })

  const oracle = priceQuery.data
  const history = historyQuery.data?.prices ?? []
  const marketDataDown = detectMarketDataOutage(priceQuery, historyQuery)

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          Protocol & Oracle
        </h1>
        <p className="text-sm mt-1 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
          Review reference pricing and hook activity used around the protocol.
        </p>
      </div>
      <p className="text-xs leading-relaxed max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
        USTC/USD reference from the indexer&apos;s polled oracle (distinct from per-pair TWAP on the Charts page). Hook
        events show burn/tax and other post-swap hooks as recorded by the indexer.
      </p>

      <div className="shell-panel" data-testid="protocol-contract-addresses">
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          On-chain contracts (audit)
        </h2>
        <p className="text-xs mb-3 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
          Factory and router addresses for this deployment ({DEFAULT_NETWORK}). Compare against your governance records
          or block explorer before signing swaps — these are not repeated on the swap confirmation card.
        </p>
        <dl className="space-y-3 text-sm">
          <div>
            <dt className="text-xs uppercase tracking-wide mb-1" style={{ color: 'var(--ink-dim)' }}>
              Factory
            </dt>
            <dd>
              {FACTORY_CONTRACT_ADDRESS ? (
                <AddressRow
                  address={FACTORY_CONTRACT_ADDRESS}
                  showFull
                  copyAriaLabel="Copy factory address"
                  explorerAriaLabel="View factory on explorer"
                  data-testid="protocol-factory-address"
                />
              ) : (
                <span style={{ color: 'var(--ink-dim)' }}>Not configured</span>
              )}
            </dd>
          </div>
          <div>
            <dt className="text-xs uppercase tracking-wide mb-1" style={{ color: 'var(--ink-dim)' }}>
              Router
            </dt>
            <dd>
              {ROUTER_CONTRACT_ADDRESS ? (
                <AddressRow
                  address={ROUTER_CONTRACT_ADDRESS}
                  showFull
                  copyAriaLabel="Copy router address"
                  explorerAriaLabel="View router on explorer"
                  data-testid="protocol-router-address"
                />
              ) : (
                <span style={{ color: 'var(--ink-dim)' }}>Not configured</span>
              )}
            </dd>
          </div>
          <div>
            <dt className="text-xs uppercase tracking-wide mb-1" style={{ color: 'var(--ink-dim)' }}>
              LCD
            </dt>
            <dd className="break-all font-mono text-xs" style={{ color: 'var(--ink-subtle)' }}>
              {TERRA_LCD_URL}
            </dd>
          </div>
          <div>
            <dt className="text-xs uppercase tracking-wide mb-1" style={{ color: 'var(--ink-dim)' }}>
              RPC
            </dt>
            <dd className="break-all font-mono text-xs" style={{ color: 'var(--ink-subtle)' }}>
              {TERRA_RPC_URL}
            </dd>
          </div>
        </dl>
      </div>

      {marketDataDown && (
        <MarketDataServiceOutageBanner
          testId="protocol-market-data-outage-banner"
          title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
          lead={PROTOCOL_MARKET_DATA_OUTAGE_LEAD}
          onRetry={() => {
            void priceQuery.refetch()
            void historyQuery.refetch()
          }}
        />
      )}

      <div className="shell-panel">
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          USTC / USD (indexer oracle)
        </h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3 mb-4">
          <StatBox
            label="Reference price"
            value={oracle?.price_usd != null ? `$${formatNum(oracle.price_usd, 6)}` : '—'}
            loading={priceQuery.isLoading}
          />
          {oracle?.sources?.[0] && (
            <>
              <StatBox label="Source" value={oracle.sources[0].source} />
              <StatBox label="Fetched" value={formatDateTime(oracle.sources[0].fetched_at)} />
            </>
          )}
        </div>
        {oracle && oracle.sources.length > 1 && (
          <div className="card-glass !p-0 overflow-x-auto">
            <table className="w-full text-xs" aria-label="Oracle sources">
              <thead>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Source</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">USD</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Time</th>
                </tr>
              </thead>
              <tbody>
                {oracle.sources.map((s) => (
                  <tr key={`${s.source}-${s.fetched_at}`} className="border-b border-white/5">
                    <td className="py-1.5 px-2 font-mono">{s.source}</td>
                    <td className="py-1.5 px-2 text-right">${formatNum(s.price_usd, 6)}</td>
                    <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                      {formatDateTime(s.fetched_at)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="shell-panel-strong">
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          Recent USTC/USD history
        </h2>
        {historyQuery.isLoading && (
          <div className="space-y-2 py-4" aria-live="polite">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} height="1.5rem" />
            ))}
          </div>
        )}
        {historyQuery.isError && (
          <RetryError message="Failed to load oracle history" onRetry={() => void historyQuery.refetch()} />
        )}
        {historyQuery.isSuccess && history.length === 0 && (
          <p className="text-sm py-4" style={{ color: 'var(--ink-dim)' }}>
            No history rows yet.
          </p>
        )}
        {history.length > 0 && (
          <div className="card-glass !p-0 overflow-x-auto max-h-64 overflow-y-auto">
            <table className="w-full text-xs" aria-label="USTC USD oracle history">
              <thead className="sticky top-0 z-10" style={{ background: 'var(--panel-bg-strong)' }}>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Time</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Price USD</th>
                </tr>
              </thead>
              <tbody>
                {history.map((row, i) => (
                  <tr key={`${row.fetched_at}-${i}`} className="border-b border-white/5">
                    <td className="py-1.5 px-2">{formatDateTime(row.fetched_at)}</td>
                    <td className="py-1.5 px-2 text-right font-mono">${formatNum(row.price_usd, 6)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="shell-panel-strong">
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          Hook events
        </h2>
        {!hooksQuery.isError && (
          <div className="flex flex-col sm:flex-row gap-2 mb-4">
            <input
              type="text"
              className="input-glass flex-1"
              placeholder="Filter by hook contract (optional)"
              value={hookFilter}
              onChange={(e) => setHookFilter(e.target.value)}
              aria-label="Filter hook events by contract"
            />
          </div>
        )}
        {hooksQuery.isLoading && (
          <div className="space-y-2 py-4" aria-live="polite">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} height="1.5rem" />
            ))}
          </div>
        )}
        {hooksQuery.isError && (
          <RetryError message="Failed to load hook events" onRetry={() => void hooksQuery.refetch()} />
        )}
        {hooksQuery.isSuccess && hooksQuery.data?.length === 0 && (
          <p className="text-sm py-4" style={{ color: 'var(--ink-dim)' }}>
            No hook events indexed yet.
          </p>
        )}
        {hooksQuery.data && hooksQuery.data.length > 0 && (
          <div className="card-glass !p-0 overflow-x-auto max-h-96 overflow-y-auto">
            <table className="w-full text-xs" aria-label="Hook events">
              <thead className="sticky top-0 z-10" style={{ background: 'var(--panel-bg-strong)' }}>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Time</th>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Hook</th>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Action</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Amount</th>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Tx</th>
                </tr>
              </thead>
              <tbody>
                {hooksQuery.data.map((h) => (
                  <tr key={h.id} className="border-b border-white/5 hover:bg-white/5">
                    <td className="py-1.5 px-2 whitespace-nowrap">{formatDateTime(h.block_time)}</td>
                    <td className="py-1.5 px-2 font-mono">{shortenAddress(h.hook_address, 8, 6)}</td>
                    <td className="py-1.5 px-2">{h.action}</td>
                    <td className="py-1.5 px-2 text-right font-mono">{formatHookAmount(h.amount)}</td>
                    <td className="py-1.5 px-2 font-mono">{shortenAddress(h.tx_hash, 6, 4)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}
