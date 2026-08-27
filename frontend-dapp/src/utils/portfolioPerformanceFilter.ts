import type { IndexerPosition, IndexerTrade } from '@/types'
import { isGemTokenId, isTestPair } from '@/utils/pairCatalogRank'

/**
 * Portfolio performance overlay for soft-launch gems (GitLab #674).
 * Reuses the #534 / #562 classifier — do not add a second gem list.
 */

export function isTestPosition(position: IndexerPosition): boolean {
  return isTestPair(
    position.asset_0_symbol,
    position.asset_1_symbol,
    position.asset_0_denom ?? undefined,
    position.asset_1_denom ?? undefined
  )
}

export function isTestTrade(trade: IndexerTrade): boolean {
  return isGemTokenId(trade.offer_asset) || isGemTokenId(trade.ask_asset)
}

export function partitionPortfolioPositions(positions: readonly IndexerPosition[]): {
  economic: IndexerPosition[]
  test: IndexerPosition[]
} {
  const economic: IndexerPosition[] = []
  const test: IndexerPosition[] = []
  for (const position of positions) {
    if (isTestPosition(position)) test.push(position)
    else economic.push(position)
  }
  return { economic, test }
}

/** Default: economic rows only. Toggle on: economic first, then gems (P674-2 / P674-4). */
export function visiblePortfolioPositions(
  positions: readonly IndexerPosition[] | undefined,
  showTestPairs: boolean
): IndexerPosition[] {
  if (!positions?.length) return []
  const { economic, test } = partitionPortfolioPositions(positions)
  return showTestPairs ? [...economic, ...test] : economic
}

export function visiblePortfolioTrades(
  trades: readonly IndexerTrade[] | undefined,
  showTestPairs: boolean
): IndexerTrade[] {
  if (!trades?.length) return []
  if (showTestPairs) return [...trades]
  return trades.filter((trade) => !isTestTrade(trade))
}

export function countTestPositions(positions: readonly IndexerPosition[] | undefined): number {
  if (!positions?.length) return 0
  return positions.filter(isTestPosition).length
}

export function countTestTrades(trades: readonly IndexerTrade[] | undefined): number {
  if (!trades?.length) return 0
  return trades.filter(isTestTrade).length
}

/** Toggle is offered when any per-pair performance row would be hidden (P674-3). */
export function shouldOfferPortfolioTestPairsToggle(
  positions: readonly IndexerPosition[] | undefined,
  trades: readonly IndexerTrade[] | undefined
): boolean {
  return countTestPositions(positions) > 0 || countTestTrades(trades) > 0
}
