import { describe, it, expect } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { useQueryManualRetry } from '../useQueryManualRetry'

function wrapper(client: QueryClient) {
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
}

describe('useQueryManualRetry', () => {
  it('forces a second fetch after error when staleTime would keep the query fresh', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    let calls = 0

    const { result } = renderHook(
      () => {
        const query = useQuery({
          queryKey: ['indexer-pair-trade', 'terra1bad'],
          queryFn: async () => {
            calls++
            throw new Error('Indexer API error: 404 Not Found')
          },
          staleTime: 60_000,
          retry: false,
        })
        const manual = useQueryManualRetry(['indexer-pair-trade', 'terra1bad'], query)
        return { query, manual }
      },
      { wrapper: wrapper(client) }
    )

    await waitFor(() => expect(result.current.query.isError).toBe(true))
    expect(calls).toBe(1)

    result.current.manual.retry()

    await waitFor(() => expect(calls).toBe(2))
    expect(result.current.manual.isRetrying).toBe(false)
  })
})
