import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { useLimitOrderPriceRefBundle } from '@/hooks/useLimitOrderPriceRefBundle'
import type { PairInfo } from '@/types'

const PAIR_ADDR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'

const pairInfo: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra19ehn7w9qxjhulu766skgequq8qjtpts6gtwekjgkg4t4ezuyhlfqr5ghcp',
  asset_infos: [
    { token: { contract_addr: 'terra1t7kqn7qlnnh0up2kf2vgkzraa2g52yzgakae2frd9r5w5qmqlr3sm3anq5' } },
    { token: { contract_addr: 'terra14n45jftyuhdxvl4t7lve5jsmzx0n92wnph6m6h73m8emsq9p6qqs6a3lmt' } },
  ],
}

function wrapper(client: QueryClient) {
  return function Wrap({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

describe('useLimitOrderPriceRefBundle integration (GitLab #166)', () => {
  it('resolves pool spot when indexer tape is unavailable', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const { result } = renderHook(
      () =>
        useLimitOrderPriceRefBundle({
          pairAddr: PAIR_ADDR,
          selectedPair: pairInfo,
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
