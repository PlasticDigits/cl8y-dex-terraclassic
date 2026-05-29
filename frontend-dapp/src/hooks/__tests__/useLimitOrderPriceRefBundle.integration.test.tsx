import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { useLimitOrderPriceRefBundle } from '@/hooks/useLimitOrderPriceRefBundle'
import {
  LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS,
  limitOrderIntegrationPairInfo,
} from '@/test/limitOrderIntegrationConstants'

function wrapper(client: QueryClient) {
  return function Wrap({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

const hasLimitOrderFixture = Boolean(import.meta.env.VITE_LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS)

describe.skipIf(!hasLimitOrderFixture)('useLimitOrderPriceRefBundle integration (GitLab #166)', () => {
  it('resolves pool spot when indexer tape is unavailable', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const { result } = renderHook(
      () =>
        useLimitOrderPriceRefBundle({
          pairAddr: LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS,
          selectedPair: limitOrderIntegrationPairInfo,
          indexerPair: null,
          latestTrade: null,
        }),
      { wrapper: wrapper(client) }
    )

    await waitFor(
      () => {
        expect(result.current.refToken1PerToken0).not.toBeNull()
        expect(result.current.refSource).toBe('pool')
      },
      { timeout: 15_000 }
    )

    expect(result.current.refResolutionError).toBe(false)
    expect(result.current.refToken1PerToken0!).toBeGreaterThan(0)
  })
})
