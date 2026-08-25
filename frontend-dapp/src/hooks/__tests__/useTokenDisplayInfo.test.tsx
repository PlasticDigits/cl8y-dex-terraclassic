import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { AssetInfo, IndexerToken } from '@/types'

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
}))

import { getTokens } from '@/services/indexer/client'
import { indexerTokenForId, useTokenDisplayInfo } from '../useTokenDisplayInfo'

const CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
const UNKNOWN_CW20 = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const TOKEN_CACHE_KEY = 'cl8y-dex-token-info'

function tokenRow(partial: Partial<IndexerToken>): IndexerToken {
  return {
    id: 1,
    contract_address: null,
    denom: null,
    is_cw20: false,
    name: partial.symbol ?? 'x',
    symbol: 'x',
    decimals: 6,
    logo_url: null,
    coingecko_id: null,
    cmc_id: null,
    ...partial,
  }
}

function wrapper(client: QueryClient) {
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
}

function renderDisplay(info: AssetInfo | null, list: IndexerToken[] | Error = []) {
  if (list instanceof Error) {
    vi.mocked(getTokens).mockRejectedValue(list)
  } else {
    vi.mocked(getTokens).mockResolvedValue(list)
  }
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  return renderHook(() => useTokenDisplayInfo(info), { wrapper: wrapper(client) })
}

describe('useTokenDisplayInfo (GitLab #630)', () => {
  beforeEach(() => {
    vi.mocked(getTokens).mockReset()
    vi.mocked(getTokens).mockResolvedValue([])
    localStorage.removeItem(TOKEN_CACHE_KEY)
  })

  afterEach(() => {
    localStorage.removeItem(TOKEN_CACHE_KEY)
  })

  it('indexer down still labels uluna / uusd as LUNC / USTC', async () => {
    const { result: lunc } = renderDisplay({ native_token: { denom: 'uluna' } }, new Error('down'))
    await waitFor(() => expect(lunc.current.symbol).toBe('LUNC'))
    expect(lunc.current.displayLabel).toBe('LUNC')

    const { result: ustc } = renderDisplay({ native_token: { denom: 'uusd' } }, new Error('down'))
    await waitFor(() => expect(ustc.current.symbol).toBe('USTC'))
  })

  it('registry wins when indexer stores denom-as-symbol or a spoof', async () => {
    const { result: denomAsSymbol } = renderDisplay({ native_token: { denom: 'uluna' } }, [
      tokenRow({ denom: 'uluna', symbol: 'uluna', name: 'uluna' }),
    ])
    await waitFor(() => expect(denomAsSymbol.current.symbol).toBe('LUNC'))

    const { result: spoof } = renderDisplay({ native_token: { denom: 'uluna' } }, [
      tokenRow({ denom: 'uluna', symbol: 'UST1', name: '<script>x</script>' }),
    ])
    await waitFor(() => expect(spoof.current.symbol).toBe('LUNC'))
    expect(spoof.current.displayLabel).toBe('LUNC')

    const { result: uusd } = renderDisplay({ native_token: { denom: 'uusd' } }, [
      tokenRow({ denom: 'uusd', symbol: 'uusd' }),
    ])
    await waitFor(() => expect(uusd.current.symbol).toBe('USTC'))
  })

  it('wrap CW20s stay cLUNC / cUSTC when indexer says LUNC-C / USTC-C (#507)', async () => {
    const { result: clunc } = renderDisplay({ token: { contract_addr: CLUNC } }, [
      tokenRow({
        contract_address: CLUNC,
        is_cw20: true,
        symbol: 'LUNC-C',
        name: 'LUNC-C',
      }),
    ])
    await waitFor(() => expect(clunc.current.symbol).toBe('cLUNC'))

    const { result: custc } = renderDisplay({ token: { contract_addr: CUSTC } }, [
      tokenRow({
        contract_address: CUSTC,
        is_cw20: true,
        symbol: 'USTC-C',
      }),
    ])
    await waitFor(() => expect(custc.current.symbol).toBe('cUSTC'))
  })

  it('unknown native stays raw unless indexer supplies a symbol', async () => {
    const { result: raw } = renderDisplay({ native_token: { denom: 'ibc/ABC' } }, [])
    await waitFor(() => expect(raw.current.symbol).toBe('ibc/ABC'))

    const { result: indexed } = renderDisplay({ native_token: { denom: 'ibc/ABC' } }, [
      tokenRow({ denom: 'ibc/ABC', symbol: 'FOO' }),
    ])
    await waitFor(() => expect(indexed.current.symbol).toBe('FOO'))
  })

  it('unknown CW20 uses indexer symbol when present', async () => {
    const { result } = renderDisplay({ token: { contract_addr: UNKNOWN_CW20 } }, [
      tokenRow({
        contract_address: UNKNOWN_CW20,
        is_cw20: true,
        symbol: 'GEMX',
      }),
    ])
    await waitFor(() => expect(result.current.symbol).toBe('GEMX'))
  })

  it('poisoned localStorage cannot relabel known natives', async () => {
    localStorage.setItem(TOKEN_CACHE_KEY, JSON.stringify({ uluna: { symbol: 'HACK', name: 'HACK' } }))
    const { result } = renderDisplay({ native_token: { denom: 'uluna' } }, [])
    await waitFor(() => expect(result.current.symbol).toBe('LUNC'))
  })

  it('indexerTokenForId does not bind a CW20 row to a native id', () => {
    const colliding = tokenRow({
      contract_address: 'uluna',
      denom: null,
      is_cw20: true,
      symbol: 'HACK',
    })
    expect(indexerTokenForId('uluna', [colliding])).toBeUndefined()

    const both = tokenRow({
      contract_address: 'uluna',
      denom: 'uluna',
      is_cw20: true,
      symbol: 'HACK',
    })
    expect(indexerTokenForId('uluna', [both])).toBeUndefined()

    const native = tokenRow({ denom: 'uluna', symbol: 'uluna' })
    expect(indexerTokenForId('uluna', [native])?.denom).toBe('uluna')
  })
})
