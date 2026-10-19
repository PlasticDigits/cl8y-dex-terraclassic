import { Fragment } from 'react'
import { formatOrderMetric } from '@/utils/routeOrderMetrics'
import { QuoteDataLoadingProgress } from '@/components/QuoteDataLoadingProgress'
import { TokenLogo } from '@/components/ui/TokenLogo'
import { tokenAssetInfo } from '@/types'
import { getTokenDisplaySymbol, getTokenLogoURI } from '@/utils/tokenDisplay'

/** Fixed summary geometry; advisory metrics never determine execution permission. */
export function SwapQuoteSummary({
  loading,
  tokens,
  orderSlippagePercent,
  tradingFeePercent,
}: {
  loading: boolean
  tokens: string[]
  orderSlippagePercent?: number | null
  tradingFeePercent?: number | null
}) {
  return (
    <div
      data-testid="swap-quote-summary"
      className="swap-quote-summary flex h-12 items-center gap-3 min-w-0 text-[11px] sm:text-xs"
      style={{ color: 'var(--ink-dim)' }}
      aria-busy={loading || undefined}
      data-long-route={tokens.length > 3 || undefined}
    >
      {loading ? (
        <div className="swap-quote-summary-loading w-full">
          <QuoteDataLoadingProgress testId="swap-price-data-loading" />
        </div>
      ) : (
        <>
          <span className="shrink-0" data-testid="swap-order-slippage">
            Slippage {formatOrderMetric(orderSlippagePercent)}
          </span>
          <span className="shrink-0" data-testid="swap-trading-fee">
            Fee {formatOrderMetric(tradingFeePercent)}
          </span>
          <div
            data-testid="swap-route-summary"
            className="swap-route-summary min-w-0 flex-1"
            role="group"
            aria-label="Selected token route"
          >
            <span className="swap-route-path flex items-center gap-1">
              {tokens.map((id, index) => (
                <Fragment key={`${index}:${id}`}>
                  {index > 0 && (
                    <>
                      <span className="swap-route-arrow shrink-0" aria-hidden="true">
                        →
                      </span>
                      <span className="sr-only"> to </span>
                    </>
                  )}
                  <span
                    className="swap-route-token inline-flex min-w-0 flex-1 items-center gap-1"
                    title={`${getTokenDisplaySymbol(id)} (${id})`}
                    data-token-identity={id}
                  >
                    <TokenLogo
                      addressForBlockie={id.startsWith('terra1') ? id : undefined}
                      blockieSeed={id}
                      logoURI={getTokenLogoURI(tokenAssetInfo(id))}
                      size={14}
                    />
                    <span className="min-w-0 truncate">{getTokenDisplaySymbol(id)}</span>
                  </span>
                </Fragment>
              ))}
              {!tokens.length && <span>—</span>}
            </span>
          </div>
        </>
      )}
    </div>
  )
}

/** Full labels and canonical identities remain readable when the compact strip truncates. */
export function SwapRouteDetails({ tokens }: { tokens: string[] }) {
  if (!tokens.length) return null
  return (
    <div data-testid="swap-route-details">
      <p className="font-medium mb-1">Route</p>
      <ol className="space-y-1" aria-label="Full selected token route">
        {tokens.map((id, index) => (
          <li key={`${index}:${id}`} className="break-words [overflow-wrap:anywhere]">
            <span>
              {index + 1}. {getTokenDisplaySymbol(id)}
            </span>{' '}
            <code className="text-[11px]" dir="ltr">
              {id}
            </code>
          </li>
        ))}
      </ol>
    </div>
  )
}
