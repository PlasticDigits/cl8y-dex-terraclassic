import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

beforeEach(() => {
  vi.stubGlobal('fetch', vi.fn())
})

afterEach(() => {
  vi.restoreAllMocks()
})

async function loadModule() {
  vi.resetModules()
  return await import('../client')
}

describe('indexer client fetchJson', () => {
  it('fetches and parses JSON', async () => {
    const mockData = { total_volume_24h: 100 }
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify(mockData), { status: 200 }))
    const client = await loadModule()
    const result = await client.getOverview()
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/overview'),
      expect.objectContaining({ signal: expect.any(AbortSignal) })
    )
    expect(result).toEqual(mockData)
  })

  it('throws on non-ok response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response('Not found', { status: 404, statusText: 'Not Found' }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response('Not found', { status: 404, statusText: 'Not Found' }))
    const client = await loadModule()
    await expect(client.getOverview()).rejects.toThrow('Indexer API error: 404')
  })

  it('throws on invalid JSON response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response('this is not json', { status: 200 }))
    const client = await loadModule()
    await expect(client.getOverview()).rejects.toThrow('invalid JSON')
  })

  it('retries on network failure then succeeds', async () => {
    vi.mocked(fetch).mockRejectedValueOnce(new Error('Failed to fetch'))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({ total_volume_24h: 50 }), { status: 200 }))
    const client = await loadModule()
    const result = await client.getOverview()
    expect(fetch).toHaveBeenCalledTimes(2)
    expect(result).toEqual({ total_volume_24h: 50 })
  })

  it('throws after max retries on network failure', async () => {
    vi.mocked(fetch).mockRejectedValueOnce(new Error('Failed to fetch'))
    vi.mocked(fetch).mockRejectedValueOnce(new Error('Failed to fetch'))
    const client = await loadModule()
    await expect(client.getOverview()).rejects.toThrow('Failed to fetch')
    expect(fetch).toHaveBeenCalledTimes(2)
  })

  it('POST /route/solve sends JSON body', async () => {
    const body = {
      token_in: 'terra1a',
      token_out: 'terra1b',
      amount_in: '100',
      hybrid_by_hop: [{ pool_input: '60', book_input: '40', max_maker_fills: 8, book_start_hint: null }],
    }
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          token_in: 'terra1a',
          token_out: 'terra1b',
          hops: [],
          router_operations: [],
          estimated_amount_out: '99',
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    const out = await client.postRouteSolve('terra1a', 'terra1b', '100', [
      { pool_input: '60', book_input: '40', max_maker_fills: 8, book_start_hint: null },
    ])
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/route/solve'),
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({ 'Content-Type': 'application/json' }),
        body: JSON.stringify(body),
      })
    )
    expect(out.estimated_amount_out).toBe('99')
  })

  it('GET /route/solve adds hybrid_optimize and max_maker_fills query params', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          token_in: 'terra1a',
          token_out: 'terra1b',
          hops: [],
          router_operations: [],
          intermediate_tokens: ['terra1a', 'terra1b'],
          quote_kind: 'indexer_hybrid_lcd',
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    await client.getRouteSolve('terra1a', 'terra1b', '1000', { hybridOptimize: true, maxMakerFills: 12 })
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).toContain('hybrid_optimize=true')
    expect(url).toContain('max_maker_fills=12')
    expect(url).toContain('amount_in=1000')
  })

  it('builds limit-fills and lifecycle URLs with query params', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    const client = await loadModule()
    const pair = 'terra1pairaddr000000000000000000000000000'
    await client.getPairLimitFills(pair, { limit: 10, before: 99 })
    await client.getPairLiquidityEvents(pair, { limit: 5 })
    await client.getPairLimitPlacements(pair)
    await client.getPairLimitCancellations(pair)
    await client.getPairOrderLimitFills(pair, 42, 20)
    expect(vi.mocked(fetch).mock.calls[0][0]).toContain(`/api/v1/pairs/${pair}/limit-fills?limit=10&before=99`)
    expect(vi.mocked(fetch).mock.calls[1][0]).toContain(`/api/v1/pairs/${pair}/liquidity-events?limit=5`)
    expect(vi.mocked(fetch).mock.calls[2][0]).toBe(`${client.INDEXER_URL}/api/v1/pairs/${pair}/limit-placements`)
    expect(vi.mocked(fetch).mock.calls[3][0]).toBe(`${client.INDEXER_URL}/api/v1/pairs/${pair}/limit-cancellations`)
    expect(vi.mocked(fetch).mock.calls[4][0]).toContain(`/api/v1/pairs/${pair}/limit-orders/42/fills?limit=20`)
  })
})
