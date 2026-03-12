import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

const INDEXER_URL = 'http://localhost:3001'

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
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(mockData), { status: 200 })
    )
    const client = await loadModule()
    const result = await client.getOverview()
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/overview'),
      expect.objectContaining({ signal: expect.any(AbortSignal) })
    )
    expect(result).toEqual(mockData)
  })

  it('throws on non-ok response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response('Not found', { status: 404, statusText: 'Not Found' })
    )
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response('Not found', { status: 404, statusText: 'Not Found' })
    )
    const client = await loadModule()
    await expect(client.getOverview()).rejects.toThrow('Indexer API error: 404')
  })

  it('throws on invalid JSON response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response('this is not json', { status: 200 })
    )
    const client = await loadModule()
    await expect(client.getOverview()).rejects.toThrow('invalid JSON')
  })

  it('retries on network failure then succeeds', async () => {
    vi.mocked(fetch).mockRejectedValueOnce(new Error('Failed to fetch'))
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify({ total_volume_24h: 50 }), { status: 200 })
    )
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
})
