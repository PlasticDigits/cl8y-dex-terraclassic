import { TradePageWorkspaceSkeleton } from '@/components/trade/TradePageWorkspaceSkeleton'

/** Suspense fallback for `/trade` lazy routes — paints before JS chunk + data (GitLab #179). */
export function TradePageRouteFallback() {
  return <TradePageWorkspaceSkeleton includePageChrome />
}
