import { useQuery } from '@tanstack/react-query'
import { useProtocolHubPricesQuery } from '@/components/protocol/useProtocolHubPricesQuery'
import { getOraclePrice } from '@/services/indexer/client'
import { traderUsdMarksFromHub, type TraderUsdMarks } from '@/utils/traderPositionDisplay'

function parseOracleUsd(priceUsd: string | null | undefined): number | null {
  if (priceUsd == null || priceUsd === '') return null
  const n = Number(priceUsd)
  return Number.isFinite(n) && n > 0 ? n : null
}

/**
 * Shared hub + CEX marks for portfolio / trader P&L (GitLab #560 / #675).
 * Same `queryKey` as Protocol DEX card. Never `$1` / `2.5×` pegs.
 */
export function useTraderUsdMarks(): TraderUsdMarks {
  const hubQuery = useProtocolHubPricesQuery()
  const ustcQuery = useQuery({
    queryKey: ['oracle-price', 'ustc'],
    queryFn: () => getOraclePrice('ustc'),
    staleTime: 60_000,
  })
  const luncQuery = useQuery({
    queryKey: ['oracle-price', 'lunc'],
    queryFn: () => getOraclePrice('lunc'),
    staleTime: 60_000,
  })
  return traderUsdMarksFromHub(hubQuery.data, {
    ustcUsd: parseOracleUsd(ustcQuery.data?.price_usd),
    luncUsd: parseOracleUsd(luncQuery.data?.price_usd),
  })
}
