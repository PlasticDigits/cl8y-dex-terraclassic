import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTrader, getTraderTrades, getTraderPositions, getTraderLimitPlacements } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, PORTFOLIO_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { TradesTable } from '@/components/ui/TradesTable'
import { TraderSummaryStats } from '@/components/trader/TraderSummaryStats'
import { TraderPositionsTable } from '@/components/trader/TraderPositionsTable'
import { PortfolioOpenLimitsSection } from '@/components/portfolio/PortfolioOpenLimitsSection'
import { PortfolioLpOverviewSection } from '@/components/portfolio/PortfolioLpOverviewSection'
import { PortfolioShowTestPairsToggle } from '@/components/portfolio/PortfolioShowTestPairsToggle'
import { usePortfolioLpBalances } from '@/hooks/usePortfolioLpBalances'
import { sounds } from '@/lib/sounds'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { formatDateTime } from '@/utils/formatDate'
import { PORTFOLIO_OPEN_LIMITS_DEFAULT_LIMIT } from '@/utils/portfolioFanOut'
import { ShareLinkButton } from '@/components/ui/ShareLinkButton'
import {
  countTestPositions,
  shouldOfferPortfolioTestPairsToggle,
  visiblePortfolioPositions,
  visiblePortfolioTrades,
} from '@/utils/portfolioPerformanceFilter'
import { buildCanonicalShareUrl, traderShareText } from '@/utils/sharePageLink'
import { SHARE_LINK_ARIA_PORTFOLIO, SHARE_LINK_TITLE } from '@/utils/sharePageLinkCopy'

export default function PortfolioPage() {
  const walletAddr = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const [showTestPairs, setShowTestPairs] = useState(false)

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

  const openLimitsQuery = useQuery({
    queryKey: ['portfolio-open-limits', walletAddr],
    queryFn: () => getTraderLimitPlacements(walletAddr!, { limit: PORTFOLIO_OPEN_LIMITS_DEFAULT_LIMIT }),
    enabled: !!walletAddr,
    refetchInterval: 30_000,
    refetchOnWindowFocus: true,
  })

  const lpQuery = usePortfolioLpBalances(walletAddr)

  const tradesQuery = useQuery({
    queryKey: ['portfolio-trades', walletAddr],
    queryFn: () => getTraderTrades(walletAddr!, { limit: 100 }),
    enabled: !!walletAddr,
    refetchInterval: 15_000,
    refetchOnWindowFocus: true,
  })

  const publicShareUrl = walletAddr
    ? buildCanonicalShareUrl({ origin: window.location.origin, kind: 'trader', id: walletAddr })
    : null

  const trader = traderQuery.data
  const visiblePositions = useMemo(
    () => visiblePortfolioPositions(positionsQuery.data, showTestPairs),
    [positionsQuery.data, showTestPairs]
  )
  const visibleTrades = useMemo(
    () => visiblePortfolioTrades(tradesQuery.data, showTestPairs),
    [tradesQuery.data, showTestPairs]
  )
  const offerTestPairsToggle = shouldOfferPortfolioTestPairsToggle(positionsQuery.data, tradesQuery.data)
  const testPairCount = countTestPositions(positionsQuery.data)
  const profileNotFound =
    traderQuery.isError && !isIndexerUnavailableError(traderQuery.error) && traderQuery.error instanceof Error
      ? traderQuery.error.message.includes('404')
      : false

  const indexerOutage =
    (traderQuery.isError && isIndexerUnavailableError(traderQuery.error)) ||
    (positionsQuery.isError && isIndexerUnavailableError(positionsQuery.error)) ||
    (openLimitsQuery.isError && isIndexerUnavailableError(openLimitsQuery.error)) ||
    (tradesQuery.isError && isIndexerUnavailableError(tradesQuery.error))

  const refetchAll = () => {
    void traderQuery.refetch()
    void positionsQuery.refetch()
    void openLimitsQuery.refetch()
    void lpQuery.refetch()
    void tradesQuery.refetch()
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          My Portfolio
        </h1>
        <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
          Wallet overview.
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
            {publicShareUrl ? (
              <ShareLinkButton
                url={publicShareUrl}
                title={SHARE_LINK_TITLE}
                text={traderShareText(walletAddr)}
                ariaLabel={SHARE_LINK_ARIA_PORTFOLIO}
                data-testid="portfolio-share-link"
              />
            ) : null}
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
              No trades yet.
            </div>
          )}

          {traderQuery.isError && !profileNotFound && !indexerOutage && (
            <RetryError message="Failed to load trader summary" onRetry={() => void traderQuery.refetch()} />
          )}

          {trader && (
            <TraderSummaryStats
              trader={trader}
              positions={positionsQuery.data === undefined ? undefined : visiblePositions}
              isOwnProfile
              addressRowTestId="portfolio-address-row"
            />
          )}

          <TraderPositionsTable
            positions={positionsQuery.data === undefined ? undefined : visiblePositions}
            isLoading={positionsQuery.isLoading}
            isError={positionsQuery.isError && !isIndexerUnavailableError(positionsQuery.error)}
            onRetry={() => void positionsQuery.refetch()}
            sectionTestId="portfolio-positions-section"
            showTestPairDivider={showTestPairs}
            headerAction={
              offerTestPairsToggle ? (
                <PortfolioShowTestPairsToggle
                  checked={showTestPairs}
                  onChange={setShowTestPairs}
                  testPairCount={testPairCount}
                />
              ) : null
            }
          />

          <PortfolioOpenLimitsSection
            placements={openLimitsQuery.data}
            isLoading={openLimitsQuery.isLoading}
            isError={openLimitsQuery.isError && !isIndexerUnavailableError(openLimitsQuery.error)}
            onRetry={() => void openLimitsQuery.refetch()}
          />

          <PortfolioLpOverviewSection
            rows={lpQuery.data?.rows}
            pairsScanned={lpQuery.data?.pairsScanned}
            capped={lpQuery.data?.capped}
            isLoading={lpQuery.isLoading}
            isError={lpQuery.isError}
            onRetry={() => void lpQuery.refetch()}
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
            {tradesQuery.data && visibleTrades.length === 0 && (
              <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>
                No trades yet.
              </p>
            )}
            {tradesQuery.data && visibleTrades.length > 0 && (
              <TradesTable trades={visibleTrades} formatTimeFn={formatDateTime} ariaLabel="Recent swap activity" />
            )}
          </div>
        </>
      )}
    </div>
  )
}
