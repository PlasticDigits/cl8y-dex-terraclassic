import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { getOracleHistory, getOraclePrice } from '@/services/indexer/client'
import type { IndexerOracleHistoryResponse, IndexerOraclePriceResponse } from '@/types'
import type { ProtocolOracleTicker } from '@/utils/protocolOracleTicker'

export function useProtocolOracleQueries(ticker: ProtocolOracleTicker): {
  priceQuery: UseQueryResult<IndexerOraclePriceResponse>
  historyQuery: UseQueryResult<IndexerOracleHistoryResponse>
} {
  const priceQuery = useQuery({
    queryKey: ['indexer-oracle-price', ticker],
    queryFn: () => getOraclePrice(ticker),
    refetchInterval: 60_000,
    retry: false,
  })

  const historyQuery = useQuery({
    queryKey: ['indexer-oracle-history', ticker],
    queryFn: () => getOracleHistory({ ticker, limit: 48 }),
    staleTime: 60_000,
    refetchInterval: 120_000,
    retry: false,
  })

  return { priceQuery, historyQuery }
}
