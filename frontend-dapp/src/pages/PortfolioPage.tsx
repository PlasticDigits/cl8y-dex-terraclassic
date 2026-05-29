import { Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTrader, getTraderTrades, getTraderPositions } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, PORTFOLIO_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { TradesTable } from '@/components/ui/TradesTable'
import { TraderSummaryStats } from '@/components/trader/TraderSummaryStats'
import { TraderPositionsTable } from '@/components/trader/TraderPositionsTable'
import { sounds } from '@/lib/sounds'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { formatDateTime } from '@/utils/formatDate'

export default function PortfolioPage() {
  const walletAddr = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)

  const traderQuery = useQuery({
    queryKey: ['portfolio-trader-profile', walletAddr],
    queryFn: () => getTrader(walletAddr!),
    enabled: !!walletAddr,
    refetchInterval: 30_000,
    retry: false,
  })

  const positionsQuery = useQuery({
    queryKey: ['portfolio-positions', walletAddr],
    queryFn: () => getTraderPositions(walletAddr!),
    enabled: !!walletAddr,
    refetchInterval: 30_000,
    refetchOnWindowFocus: true,
  })

  const tradesQuery = useQuery({
    queryKey: ['portfolio-trades', walletAddr],
    queryFn: () => getTraderTrades(walletAddr!, { limit: 100 }),
    enabled: !!walletAddr,
    refetchInterval: 15_000,
    refetchOnWindowFocus: true,
  })

  const trader = traderQuery.data
  const profileNotFound =
    traderQuery.isError && !isIndexerUnavailableError(traderQuery.error) && traderQuery.error instanceof Error
      ? traderQuery.error.message.includes('404')
      : false

  const indexerOutage =
    (traderQuery.isError && isIndexerUnavailableError(traderQuery.error)) ||
    (positionsQuery.isError && isIndexerUnavailableError(positionsQuery.error)) ||
    (tradesQuery.isError && isIndexerUnavailableError(tradesQuery.error))

  const refetchAll = () => {
    void traderQuery.refetch()
    void positionsQuery.refetch()
    void tradesQuery.refetch()
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          My Portfolio
        </h1>
        <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
          Your connected wallet&apos;s indexed trading exposure, realized P&amp;L, and recent swaps — read-only; no
          signing on this page.
        </p>
      </div>

      {!walletAddr && (
        <div className="shell-panel-strong text-center py-12 space-y-4" data-testid="portfolio-connect-prompt">
          <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
            Connect your wallet to view your portfolio.
          </p>
          <button
            type="button"
            className="btn-primary btn-cta !px-6 !py-2"
            onClick={() => {
              sounds.playButtonPress()
              openWalletModal()
            }}
          >
            Connect Wallet
          </button>
        </div>
      )}

      {walletAddr && (
        <>
          <div className="flex flex-wrap items-center gap-2 text-sm" style={{ color: 'var(--ink-dim)' }}>
            <span>Public trader profile:</span>
            <Link
              to={`/trader/${walletAddr}`}
              onClick={() => sounds.playButtonPress()}
              className="underline font-medium"
              style={{ color: 'var(--accent)' }}
              data-testid="portfolio-public-profile-link"
            >
              View on Trader page
            </Link>
          </div>

          {indexerOutage && (
            <MarketDataServiceOutageBanner
              testId="portfolio-market-data-outage-banner"
              title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
              lead={PORTFOLIO_MARKET_DATA_OUTAGE_LEAD}
              onRetry={refetchAll}
            />
          )}

          {traderQuery.isLoading && (
            <div
              className="shell-panel-strong space-y-3 py-6"
              aria-live="polite"
              data-testid="portfolio-summary-loading"
            >
              <Skeleton height="1rem" width="40%" />
              <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                {Array.from({ length: 4 }).map((_, i) => (
                  <Skeleton key={i} height="3rem" />
                ))}
              </div>
            </div>
          )}

          {profileNotFound && (
            <div
              className="shell-panel-strong text-center py-6 text-sm"
              style={{ color: 'var(--ink-dim)' }}
              data-testid="portfolio-profile-empty"
            >
              No indexed trades yet for this wallet. Open positions below may still appear after swaps are indexed.
            </div>
          )}

          {traderQuery.isError && !profileNotFound && !indexerOutage && (
            <RetryError message="Failed to load trader summary" onRetry={() => void traderQuery.refetch()} />
          )}

          {trader && <TraderSummaryStats trader={trader} isOwnProfile addressRowTestId="portfolio-address-row" />}

          <TraderPositionsTable
            positions={positionsQuery.data}
            isLoading={positionsQuery.isLoading}
            isError={positionsQuery.isError && !isIndexerUnavailableError(positionsQuery.error)}
            onRetry={() => void positionsQuery.refetch()}
            sectionTestId="portfolio-positions-section"
          />

          <div className="shell-panel-strong" data-testid="portfolio-recent-activity">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
              style={{ color: 'var(--ink)' }}
            >
              Recent activity
            </h3>
            {tradesQuery.isLoading && (
              <div className="space-y-2 py-4" aria-live="polite">
                {Array.from({ length: 5 }).map((_, i) => (
                  <Skeleton key={i} height="1.5rem" />
                ))}
              </div>
            )}
            {tradesQuery.isError && !isIndexerUnavailableError(tradesQuery.error) && (
              <RetryError message="Failed to load recent swaps" onRetry={() => void tradesQuery.refetch()} />
            )}
            {tradesQuery.data && tradesQuery.data.length === 0 && (
              <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>
                No indexed swaps yet.
              </p>
            )}
            {tradesQuery.data && tradesQuery.data.length > 0 && (
              <TradesTable trades={tradesQuery.data} formatTimeFn={formatDateTime} ariaLabel="Recent swap activity" />
            )}
          </div>
        </>
      )}
    </div>
  )
}
