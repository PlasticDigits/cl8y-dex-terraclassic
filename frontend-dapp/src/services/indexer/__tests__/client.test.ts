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
