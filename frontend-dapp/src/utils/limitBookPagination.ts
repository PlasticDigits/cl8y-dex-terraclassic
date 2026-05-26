/** Default rows per indexer `limit-book` page in the dApp (≤ indexer max 100). */
export const LIMIT_BOOK_UI_PAGE_SIZE = 45

export function limitBookPageQueryKey(pairAddress: string, side: 'bid' | 'ask') {
  return ['limitBookPage', pairAddress, side] as const
}
