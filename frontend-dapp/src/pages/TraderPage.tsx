import { useState, useId } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTrader, getTraderTrades, getTraderPositions } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, TRADER_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { TradesTable } from '@/components/ui/TradesTable'
import { TraderSummaryStats } from '@/components/trader/TraderSummaryStats'
import { TraderPositionsTable } from '@/components/trader/TraderPositionsTable'
import { ShareLinkButton } from '@/components/ui/ShareLinkButton'
import { TraderLeaderboard } from '@/components/trader/TraderLeaderboard'
import { sounds } from '@/lib/sounds'
import { isValidTerraAddress } from '@/utils/constants'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { formatDateTime } from '@/utils/formatDate'
import { buildCanonicalShareUrl, traderShareText } from '@/utils/sharePageLink'
import { SHARE_LINK_ARIA_TRADER, SHARE_LINK_TITLE } from '@/utils/sharePageLinkCopy'

export default function TraderPage() {
  const { address: paramAddr } = useParams<{ address?: string }>()
  const navigate = useNavigate()
  const walletAddr = useWalletStore((s) => s.address)
  const traderWalletSearchInputId = useId()
  const [searchInput, setSearchInput] = useState('')

  const traderAddr = paramAddr || ''
  const shareUrl = buildCanonicalShareUrl({
    origin: window.location.origin,
    kind: 'trader',
    id: traderAddr,
  })

  const traderQuery = useQuery({
    queryKey: ['trader-profile', traderAddr],
    queryFn: () => getTrader(traderAddr),
    enabled: !!traderAddr,
    refetchInterval: 30_000,
    retry: false,
  })

  const tradesQuery = useQuery({
    queryKey: ['trader-trades', traderAddr],
    queryFn: () => getTraderTrades(traderAddr, { limit: 100 }),
    enabled: !!traderAddr,
    refetchInterval: 15_000,
  })

  const positionsQuery = useQuery({
    queryKey: ['trader-positions', traderAddr],
    queryFn: () => getTraderPositions(traderAddr),
    enabled: !!traderAddr,
    refetchInterval: 30_000,
  })

  const trader = traderQuery.data
  const isOwnProfile = walletAddr && walletAddr === traderAddr

  const searchTrimmed = searchInput.trim()

  const handleSearch = () => {
    const addr = searchTrimmed
    if (addr && isValidTerraAddress(addr)) {
      sounds.playButtonPress()
      navigate(`/trader/${addr}`)
      setSearchInput('')
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
            Trader Profile
          </h1>
          <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
            Look up a wallet to review trading activity, positions, and P&amp;L.
          </p>
        </div>
        {shareUrl ? (
          <ShareLinkButton
            url={shareUrl}
            title={SHARE_LINK_TITLE}
            text={traderShareText(traderAddr)}
            ariaLabel={SHARE_LINK_ARIA_TRADER}
            data-testid="trader-share-link"
          />
        ) : null}
      </div>

      {/* Search / My Profile */}
      <div className="shell-panel flex flex-col sm:flex-row gap-2">
        <div className="flex gap-2 flex-1">
          <label htmlFor={traderWalletSearchInputId} className="sr-only">
            Trader wallet address
          </label>
          <input
            id={traderWalletSearchInputId}
            type="text"
            className="input-glass flex-1"
            placeholder="Paste a trader wallet address"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          />
          <button className="btn-primary btn-cta !px-4 !py-1.5 !text-xs" onClick={handleSearch}>
            Search
          </button>
        </div>
        {walletAddr && (
          <>
            <Link
              to={`/trader/${walletAddr}`}
              onClick={() => sounds.playButtonPress()}
              className="btn-primary btn-cta !px-4 !py-1.5 !text-xs text-center no-underline self-start sm:self-auto"
            >
              My Profile
            </Link>
            <Link
              to="/portfolio"
              onClick={() => sounds.playButtonPress()}
              className="btn-primary btn-cta !px-4 !py-1.5 !text-xs text-center no-underline self-start sm:self-auto"
            >
              My Portfolio
            </Link>
          </>
        )}
      </div>

      {!traderAddr && (
        <div className="shell-panel-strong text-center py-12">
          <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
            Search for a trader wallet above, or open your own profile once your wallet is connected.
          </p>
        </div>
      )}

      {traderAddr && traderQuery.isLoading && (
        <div className="shell-panel-strong space-y-3 py-6" aria-live="polite">
          <Skeleton height="1rem" width="40%" />
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} height="3rem" />
            ))}
          </div>
        </div>
      )}

      {traderAddr && traderQuery.isError && isIndexerUnavailableError(traderQuery.error) && (
        <MarketDataServiceOutageBanner
          testId="trader-market-data-outage-banner"
          title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
          lead={TRADER_MARKET_DATA_OUTAGE_LEAD}
          onRetry={() => void traderQuery.refetch()}
        />
      )}

      {traderAddr && traderQuery.isError && !isIndexerUnavailableError(traderQuery.error) && (
        <RetryError
          message="Trader not found. They may not have traded yet."
          onRetry={() => void traderQuery.refetch()}
        />
      )}

      {trader && (
        <>
          <TraderSummaryStats trader={trader} positions={positionsQuery.data} isOwnProfile={!!isOwnProfile} />

          <TraderPositionsTable
            positions={positionsQuery.data}
            isLoading={positionsQuery.isLoading}
            isError={positionsQuery.isError}
            onRetry={() => void positionsQuery.refetch()}
            emptyMessage="No positions"
          />

          <div className="shell-panel-strong">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
              style={{ color: 'var(--ink)' }}
            >
              Trade History
            </h3>
            {tradesQuery.isLoading && (
              <div className="space-y-2 py-4" aria-live="polite">
                {Array.from({ length: 5 }).map((_, i) => (
                  <Skeleton key={i} height="1.5rem" />
                ))}
              </div>
            )}
            {tradesQuery.isError && (
              <RetryError message="Failed to load trades" onRetry={() => void tradesQuery.refetch()} />
            )}
            {tradesQuery.data && (
              <TradesTable trades={tradesQuery.data} formatTimeFn={formatDateTime} ariaLabel="Trade history" />
            )}
          </div>
        </>
      )}

      {/* Last page section (TL-1). Outside profile gates so empty / 404 / outage still show the board (TL-2). */}
      <TraderLeaderboard highlightAddress={traderAddr || undefined} />
    </div>
  )
}
