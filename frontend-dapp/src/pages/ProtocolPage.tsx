import { useState, useDeferredValue } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useSearchParams } from 'react-router-dom'
import { getHookEvents } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { RetryError, Skeleton } from '@/components/ui'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, PROTOCOL_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import { detectMarketDataOutage } from '@/utils/marketDataOutage'
import { shortenAddress } from '@/utils/tokenDisplay'
import { formatDateTime } from '@/utils/formatDate'
import { AddressRow } from '@/components/ui/AddressRow'
import { ProtocolGlobalStats } from '@/components/protocol/ProtocolGlobalStats'
import { ProtocolDexHubPrices } from '@/components/protocol/ProtocolDexHubPrices'
import { ProtocolOracleCard } from '@/components/protocol/ProtocolOracleCard'
import { useProtocolOracleQueries } from '@/components/protocol/useProtocolOracleQueries'
import { useProtocolOverviewQuery } from '@/components/protocol/useProtocolOverviewQuery'
import { useProtocolHubPricesQuery } from '@/components/protocol/useProtocolHubPricesQuery'
import { parseProtocolOracleTicker, type ProtocolOracleTicker } from '@/utils/protocolOracleTicker'
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
  const [searchParams, setSearchParams] = useSearchParams()
  const ticker = parseProtocolOracleTicker(searchParams.get('ticker'))

  function setTicker(next: ProtocolOracleTicker) {
    setSearchParams(
      (prev) => {
        const nextParams = new URLSearchParams(prev)
        if (next === 'ustc') nextParams.delete('ticker')
        else nextParams.set('ticker', next)
        return nextParams
      },
      { replace: true }
    )
  }

  const overviewQuery = useProtocolOverviewQuery()
  const hubPricesQuery = useProtocolHubPricesQuery()
  const { priceQuery, historyQuery } = useProtocolOracleQueries(ticker)

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

  const marketDataDown = detectMarketDataOutage(overviewQuery, hubPricesQuery, priceQuery, historyQuery)

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          Protocol
        </h1>
        <p className="text-sm mt-1 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
          DEX USD stats, DEX hub marks, and CEX reference prices. Not settlement.
        </p>
      </div>

      {marketDataDown && (
        <MarketDataServiceOutageBanner
          testId="protocol-market-data-outage-banner"
          title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
          lead={PROTOCOL_MARKET_DATA_OUTAGE_LEAD}
          onRetry={() => {
            void overviewQuery.refetch()
            void hubPricesQuery.refetch()
            void priceQuery.refetch()
            void historyQuery.refetch()
          }}
        />
      )}

      <ProtocolGlobalStats overviewQuery={overviewQuery} />
      <ProtocolDexHubPrices query={hubPricesQuery} />
      <ProtocolOracleCard
        ticker={ticker}
        onTickerChange={setTicker}
        priceQuery={priceQuery}
        historyQuery={historyQuery}
      />

      <div className="shell-panel" data-testid="protocol-contract-addresses">
        <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          On-chain contracts (audit)
        </h2>
        <p className="text-xs mb-3 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
          Factory and router for this deployment ({DEFAULT_NETWORK}). Compare against governance records before signing.
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
