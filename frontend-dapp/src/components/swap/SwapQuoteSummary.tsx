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
      className="flex h-12 items-center gap-3 min-w-0 text-[11px] sm:text-xs"
      style={{ color: 'var(--ink-dim)' }}
      aria-busy={loading || undefined}
    >
      {loading ? (
        <div className="w-full">
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
            className="min-w-0 flex-1 overflow-x-auto"
            tabIndex={tokens.length ? 0 : undefined}
            role="group"
            aria-label="Selected token route"
          >
            <span className="inline-flex items-center gap-1 whitespace-nowrap">
              {tokens.map((id, index) => (
                <Fragment key={`${index}:${id}`}>
                  {index > 0 && <span aria-hidden="true">→</span>}
                  <span className="inline-flex items-center gap-1" title={id} data-token-identity={id}>
                    <TokenLogo
                      addressForBlockie={id.startsWith('terra1') ? id : undefined}
                      blockieSeed={id}
                      logoURI={getTokenLogoURI(tokenAssetInfo(id))}
                      size={14}
                    />
                    <span className="max-w-20 truncate">{getTokenDisplaySymbol(id)}</span>
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
