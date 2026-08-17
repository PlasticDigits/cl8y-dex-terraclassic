import type { IndexerPair, IndexerPairSort } from '@/types'
import { sortIndexerPairsByCatalog } from '@/utils/pairCatalogRank'

/** Visible page size for `/pool` table pagination (GitLab #547). */
export const POOL_PAGE_SIZE = 20

/**
 * Catalog default fetches a large `volume_24h` window then client-ranks (P534).
 * Indexer `GET /api/v1/pairs` max limit is 1000; 500 covers current catalogs
 * without an indexer `sort=catalog` API.
 */
export const POOL_CATALOG_FETCH_LIMIT = 500

/** Sortable table columns that map 1:1 to indexer `sort=` keys (A5). */
export const POOL_COLUMN_SORTS = ['symbol', 'volume_24h', 'fee', 'created'] as const

export type PoolColumnSort = (typeof POOL_COLUMN_SORTS)[number]

export type PoolListMode = 'catalog' | 'search' | 'column'

const INDEXER_PAIR_SORTS: readonly IndexerPairSort[] = ['id', 'fee', 'created', 'symbol', 'volume_24h', 'relevance']

export function isIndexerPairSort(value: string): value is IndexerPairSort {
  return (INDEXER_PAIR_SORTS as readonly string[]).includes(value)
}

export function isPoolColumnSort(value: string): value is PoolColumnSort {
  return (POOL_COLUMN_SORTS as readonly string[]).includes(value)
}

/** Volume/fee/created default desc; name/id default asc. */
export function defaultOrderForPoolSort(sort: PoolColumnSort | 'id'): 'asc' | 'desc' {
  return sort === 'symbol' || sort === 'id' ? 'asc' : 'desc'
}

export function paginatePoolPairs<T>(items: readonly T[], page: number, pageSize = POOL_PAGE_SIZE): T[] {
  const start = Math.max(0, page) * pageSize
  return items.slice(start, start + pageSize)
}

export function catalogRankAndPaginate(
  items: readonly IndexerPair[],
  page: number,
  pageSize = POOL_PAGE_SIZE
): { pageItems: IndexerPair[]; total: number } {
  const ranked = sortIndexerPairsByCatalog([...items])
  return { pageItems: paginatePoolPairs(ranked, page, pageSize), total: ranked.length }
}
