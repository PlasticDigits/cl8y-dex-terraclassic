import { formatPairPrice } from '@/utils/formatAmount'
import { AddressRow } from '@/components/ui/AddressRow'
import { RetryError, Skeleton } from '@/components/ui'
import type { UseQueryResult } from '@tanstack/react-query'
import type { IndexerHubPricesResponse } from '@/types'
import {
  HUB_PRICE_TICKERS,
  HUB_PRICE_TICKER_LABEL,
  isHubOracleWrapTicker,
  type HubPriceTicker,
} from '@/utils/hubPriceTicker'
import { resolveHubOracleWrapAddress } from '@/utils/hubOracleWrapAddress'

function formatHubUsd(raw: string | null | undefined): string {
  if (raw == null || raw === '') return '—'
  const n = Number(raw)
  if (!Number.isFinite(n) || n <= 0) return '—'
  return `$${formatPairPrice(raw)}`
}

function wrapAriaName(ticker: HubPriceTicker): string {
  return ticker === 'lunc' ? 'cLUNC wrap' : `${HUB_PRICE_TICKER_LABEL[ticker]} token`
}

interface ProtocolDexHubPricesProps {
  query: UseQueryResult<IndexerHubPricesResponse>
}

export function ProtocolDexHubPrices({ query }: ProtocolDexHubPricesProps) {
  const byTicker = new Map((query.data?.prices ?? []).map((p) => [p.ticker, p]))

  return (
    <div className="shell-panel" data-testid="protocol-dex-hub-prices">
      <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
        DEX hub prices
      </h2>
      <p className="text-xs mb-3 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
        DEX reference — not CEX, not settlement.
      </p>

      {query.isError && <RetryError message="Failed to load DEX hub prices" onRetry={() => void query.refetch()} />}
      <dl className="grid gap-3 grid-cols-1 sm:grid-cols-2 xl:grid-cols-4">
        {HUB_PRICE_TICKERS.map((ticker: HubPriceTicker) => {
          const row = byTicker.get(ticker)
          const source = row?.source_pair
          const wrap = isHubOracleWrapTicker(ticker) ? resolveHubOracleWrapAddress(ticker, row?.asset_address) : null
          const wrapName = wrapAriaName(ticker)
          return (
            <div key={ticker} data-testid={`protocol-dex-hub-${ticker}`}>
              <dt className="text-xs uppercase tracking-wide mb-1" style={{ color: 'var(--ink-dim)' }}>
                {HUB_PRICE_TICKER_LABEL[ticker]} / USD
              </dt>
              <dd
                className="text-lg font-semibold tabular-nums"
                style={{ color: 'var(--ink)' }}
                data-testid={`protocol-dex-hub-${ticker}-usd`}
              >
                {query.isLoading && !query.data ? <Skeleton height="1.75rem" /> : formatHubUsd(row?.price_usd)}
              </dd>
              {wrap ? (
                <div className="mt-1 min-w-0">
                  <AddressRow
                    address={wrap}
                    startChars={6}
                    endChars={4}
                    copyAriaLabel={`Copy ${wrapName} contract`}
                    explorerAriaLabel={`View ${wrapName} contract on explorer`}
                    data-testid={`protocol-dex-hub-${ticker}-token`}
                  />
                </div>
              ) : null}
              {source ? (
                <div className="mt-1 min-w-0">
                  <AddressRow
                    address={source}
                    startChars={6}
                    endChars={4}
                    copyAriaLabel={`Copy ${HUB_PRICE_TICKER_LABEL[ticker]} source pair`}
                    explorerAriaLabel={`View ${HUB_PRICE_TICKER_LABEL[ticker]} source pair on explorer`}
                    data-testid={`protocol-dex-hub-${ticker}-source`}
                  />
                </div>
              ) : null}
            </div>
          )
        })}
      </dl>
    </div>
  )
}
