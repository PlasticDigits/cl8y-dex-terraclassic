import type { KeyboardEvent } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { StatBox, RetryError, Skeleton } from '@/components/ui'
import { formatDateTime } from '@/utils/formatDate'
import { formatProtocolOracleUsd } from '@/utils/formatProtocolStats'
import {
  PROTOCOL_ORACLE_TICKER_LABEL,
  PROTOCOL_ORACLE_TICKERS,
  type ProtocolOracleTicker,
} from '@/utils/protocolOracleTicker'
import { sounds } from '@/lib/sounds'
import type { IndexerOracleHistoryResponse, IndexerOraclePriceResponse } from '@/types'

interface ProtocolOracleCardProps {
  ticker: ProtocolOracleTicker
  onTickerChange: (ticker: ProtocolOracleTicker) => void
  priceQuery: UseQueryResult<IndexerOraclePriceResponse>
  historyQuery: UseQueryResult<IndexerOracleHistoryResponse>
}

export function ProtocolOracleCard({ ticker, onTickerChange, priceQuery, historyQuery }: ProtocolOracleCardProps) {
  const label = PROTOCOL_ORACLE_TICKER_LABEL[ticker]
  const heading = `${label} / USD`

  const oracle = priceQuery.data
  const history = historyQuery.data?.prices ?? []

  function selectTicker(next: ProtocolOracleTicker) {
    if (next === ticker) return
    sounds.playButtonPress()
    onTickerChange(next)
  }

  function onTabListKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    const idx = PROTOCOL_ORACLE_TICKERS.indexOf(ticker)
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
      e.preventDefault()
      selectTicker(PROTOCOL_ORACLE_TICKERS[(idx + 1) % PROTOCOL_ORACLE_TICKERS.length])
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
      e.preventDefault()
      selectTicker(PROTOCOL_ORACLE_TICKERS[(idx - 1 + PROTOCOL_ORACLE_TICKERS.length) % PROTOCOL_ORACLE_TICKERS.length])
    } else if (e.key === 'Home') {
      e.preventDefault()
      selectTicker(PROTOCOL_ORACLE_TICKERS[0])
    } else if (e.key === 'End') {
      e.preventDefault()
      selectTicker(PROTOCOL_ORACLE_TICKERS[PROTOCOL_ORACLE_TICKERS.length - 1])
    }
  }

  return (
    <div className="shell-panel" data-testid="protocol-oracle">
      <h2
        className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
        style={{ color: 'var(--ink)' }}
        id="protocol-oracle-heading"
      >
        {heading}
      </h2>
      <p className="text-xs mb-3 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
        Indexer CEX/aggregator reference — not swap settlement, TWAP, or the UST1 window rate.
      </p>

      <div
        className="flex gap-2 mb-4 flex-wrap"
        role="tablist"
        aria-label="Oracle ticker"
        data-testid="protocol-oracle-tabs"
        onKeyDown={onTabListKeyDown}
      >
        {PROTOCOL_ORACLE_TICKERS.map((value) => {
          const active = ticker === value
          return (
            <button
              key={value}
              type="button"
              role="tab"
              aria-selected={active}
              tabIndex={active ? 0 : -1}
              data-testid={`protocol-oracle-tab-${value}`}
              className={`flex-1 min-w-[5.5rem] py-2.5 text-sm font-semibold uppercase tracking-wide ${
                active ? 'btn-primary' : ''
              }`}
              style={
                active
                  ? undefined
                  : {
                      background: 'var(--panel-muted, transparent)',
                      color: 'var(--ink-dim)',
                      border: '1px solid var(--stroke, transparent)',
                    }
              }
              onClick={() => selectTicker(value)}
            >
              {PROTOCOL_ORACLE_TICKER_LABEL[value]}
            </button>
          )
        })}
      </div>

      {priceQuery.isError && (
        <RetryError message="Failed to load oracle price" onRetry={() => void priceQuery.refetch()} />
      )}
      {!priceQuery.isError && (
        <>
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3 mb-4">
            <StatBox
              label="Reference price"
              value={formatProtocolOracleUsd(oracle?.price_usd)}
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
            <div className="card-glass !p-0 overflow-x-auto mb-4">
              <table className="w-full text-xs" aria-label={`${label} oracle sources`}>
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
                      <td className="py-1.5 px-2 text-right">{formatProtocolOracleUsd(s.price_usd)}</td>
                      <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                        {formatDateTime(s.fetched_at)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      <h3
        className="text-xs font-semibold uppercase tracking-wide mb-2 font-heading"
        style={{ color: 'var(--ink-dim)' }}
      >
        Recent history
      </h3>
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
        <p className="text-sm py-4" style={{ color: 'var(--ink-dim)' }} data-testid="protocol-oracle-history-empty">
          No history
        </p>
      )}
      {history.length > 0 && (
        <div className="card-glass !p-0 overflow-x-auto max-h-64 overflow-y-auto">
          <table className="w-full text-xs" aria-label={`${label} USD oracle history`}>
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
                  <td className="py-1.5 px-2 text-right font-mono">{formatProtocolOracleUsd(row.price_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
