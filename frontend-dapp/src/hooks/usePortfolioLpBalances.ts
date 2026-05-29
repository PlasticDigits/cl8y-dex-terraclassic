import { useQuery } from '@tanstack/react-query'
import { getPairs } from '@/services/indexer/client'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { assetInfoLabel, indexerPairToPairInfo, tokenAssetInfo } from '@/types'
import { isValidTerraAddress } from '@/utils/constants'
import { mapWithConcurrency } from '@/utils/mapWithConcurrency'
import { PORTFOLIO_LP_CONCURRENCY, PORTFOLIO_LP_MAX_PAIRS } from '@/utils/portfolioFanOut'

export type PortfolioLpRow = {
  pairAddress: string
  label: string
  lpToken: string
  balanceRaw: string
}

export async function fetchPortfolioLpRowsForTest(walletAddr: string): Promise<{
  rows: PortfolioLpRow[]
  pairsScanned: number
  capped: boolean
}> {
  const list = await getPairs({ limit: PORTFOLIO_LP_MAX_PAIRS, offset: 0 })
  const pairs = list.items.filter(
    (p) =>
      p.lp_token?.trim() &&
      isValidTerraAddress(p.pair_address) &&
      isValidTerraAddress(p.lp_token.trim())
  )
  const capped = list.total > PORTFOLIO_LP_MAX_PAIRS

  const balances = await mapWithConcurrency(pairs, PORTFOLIO_LP_CONCURRENCY, async (p) => {
    let pairInfo
    try {
      pairInfo = indexerPairToPairInfo(p)
    } catch {
      return null
    }
    const lpToken = pairInfo.liquidity_token
    if (!lpToken || !isValidTerraAddress(lpToken)) return null
    try {
      const balanceRaw = await getTokenBalance(walletAddr, tokenAssetInfo(lpToken))
      return {
        pairAddress: p.pair_address,
        label: `${p.asset_0.symbol ?? assetInfoLabel(pairInfo.asset_infos[0])}/${p.asset_1.symbol ?? assetInfoLabel(pairInfo.asset_infos[1])}`,
        lpToken,
        balanceRaw,
      }
    } catch {
      return null
    }
  })

  const rows = balances.filter((r): r is PortfolioLpRow => r != null && r.balanceRaw !== '0' && r.balanceRaw !== '')
  return { rows, pairsScanned: pairs.length, capped }
}

export function usePortfolioLpBalances(walletAddr: string | null | undefined) {
  return useQuery({
    queryKey: ['portfolio-lp-balances', walletAddr],
    queryFn: () => fetchPortfolioLpRowsForTest(walletAddr!),
    enabled: !!walletAddr,
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  })
}
