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
      trader: null,
      sender: null,
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

  it('GET /route/solve sends trader when connected wallet provided (GitLab #245)', async () => {
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
    const trader = 'terra1wallet000000000000000000000000000000'
    await client.getRouteSolve('terra1a', 'terra1b', '1000', { trader })
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).toContain(`trader=${trader}`)
  })

  it('POST /route/solve includes trader in JSON body (GitLab #245)', async () => {
    const trader = 'terra1wallet000000000000000000000000000000'
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
    await client.postRouteSolve(
      'terra1a',
      'terra1b',
      '100',
      [{ pool_input: '100', book_input: '0', max_maker_fills: 1, book_start_hint: null }],
      { trader }
    )
    const body = JSON.parse((vi.mocked(fetch).mock.calls[0][1] as RequestInit).body as string)
    expect(body.trader).toBe(trader)
  })

  it('GET /route/solve sends amount_in and max_maker_fills by default (hybrid-aware GET)', async () => {
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
    await client.getRouteSolve('terra1a', 'terra1b', '1000', { maxMakerFills: 12 })
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).not.toContain('hybrid_optimize=')
    expect(url).not.toContain('pool_only=')
    expect(url).toContain('max_maker_fills=12')
    expect(url).toContain('amount_in=1000')
  })

  it('GET /route/solve adds pool_only when requested', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          token_in: 'terra1a',
          token_out: 'terra1b',
          hops: [],
          router_operations: [],
          intermediate_tokens: ['terra1a', 'terra1b'],
          quote_kind: 'indexer_pool_lcd',
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    await client.getRouteSolve('terra1a', 'terra1b', '1000', { poolOnly: true })
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).toContain('pool_only=true')
  })

  it('encodes pair and trader path segments (GitLab #379)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          address: 'terra1trader000000000000000000000000000000',
          total_volume: '0',
          total_trades: 0,
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    const pair = 'terra1pair/with/slashes'
    const trader = 'terra1trader+special'
    await client.getPair(pair)
    await client.getTrader(trader)
    expect(vi.mocked(fetch).mock.calls[0][0]).toContain(`/api/v1/pairs/${encodeURIComponent(pair)}`)
    expect(vi.mocked(fetch).mock.calls[1][0]).toContain(`/api/v1/traders/${encodeURIComponent(trader)}`)
  })

  it('builds limit-fills and lifecycle URLs with query params', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    const client = await loadModule()
    const pair = 'terra1pairaddr000000000000000000000000000'
    const encPair = encodeURIComponent(pair)
    await client.getPairLimitFills(pair, { limit: 10, before: 99 })
    await client.getPairLiquidityEvents(pair, { limit: 5 })
    await client.getPairLimitPlacements(pair)
    await client.getPairLimitCancellations(pair)
    await client.getPairOrderLimitFills(pair, 42, 20)
    expect(vi.mocked(fetch).mock.calls[0][0]).toContain(`/api/v1/pairs/${encPair}/limit-fills?limit=10&before=99`)
    expect(vi.mocked(fetch).mock.calls[1][0]).toContain(`/api/v1/pairs/${encPair}/liquidity-events?limit=5`)
    expect(vi.mocked(fetch).mock.calls[2][0]).toBe(`${client.INDEXER_URL}/api/v1/pairs/${encPair}/limit-placements`)
    expect(vi.mocked(fetch).mock.calls[3][0]).toBe(`${client.INDEXER_URL}/api/v1/pairs/${encPair}/limit-cancellations`)
    expect(vi.mocked(fetch).mock.calls[4][0]).toContain(`/api/v1/pairs/${encPair}/limit-orders/42/fills?limit=20`)
  })

  it('adds status query param to limit-placements when provided', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    const client = await loadModule()
    const pair = 'terra1pair000000000000000000000000000000'
    await client.getPairLimitPlacements(pair, { status: 'parked_expired', limit: 20 })
    expect(vi.mocked(fetch).mock.calls[0][0]).toContain(`${pair}/limit-placements?limit=20&status=parked_expired`)
  })

  it('GET /traders/{addr}/positions uses encoded address path (GitLab #212)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    const client = await loadModule()
    const addr = 'terra1trader000000000000000000000000000000'
    const encAddr = encodeURIComponent(addr)
    await client.getTraderPositions(addr)
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe(`${client.INDEXER_URL}/api/v1/traders/${encAddr}/positions`)
  })

  it('GET /traders/{addr}/limit-placements supports status and pair filters (GitLab #217)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify([]), { status: 200 }))
    const client = await loadModule()
    const addr = 'terra1trader000000000000000000000000000000'
    const pair = 'terra1pair000000000000000000000000000000'
    const encAddr = encodeURIComponent(addr)
    await client.getTraderLimitPlacements(addr, { limit: 50, status: 'active', pair })
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).toBe(
      `${client.INDEXER_URL}/api/v1/traders/${encAddr}/limit-placements?limit=50&status=active&pair=${pair}`
    )
  })

  it('builds paginated limit-book URL', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          side: 'ask',
          orders: [],
          has_more: false,
          next_after_order_id: null,
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    const pair = 'terra1pairaddr000000000000000000000000000'
    await client.getPairLimitBookPage(pair, 'ask', { limit: 50, afterOrderId: 42 })
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).toContain(`/api/v1/pairs/${pair}/limit-book?`)
    expect(url).toContain('side=ask')
    expect(url).toContain('limit=50')
    expect(url).toContain('after_order_id=42')
  })

  it('normalizes limit-book pages missing orders (GitLab #327)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          side: 'bid',
          has_more: false,
          next_after_order_id: null,
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    const page = await client.getPairLimitBookPage('terra1pairaddr000000000000000000000000000', 'bid')
    expect(page.orders).toEqual([])
    expect(page.has_more).toBe(false)
  })

  it('builds insert-hints URL (GitLab #267 / #268)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          side: 'bid',
          hints: [{ price: '1', predecessor_order_id: null, resolved: true, reason: 'head' }],
          budget_exhausted: false,
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    const pair = 'terra1pairaddr000000000000000000000000000'
    await client.getPairLimitBookInsertHints(pair, 'bid', ['0.95', '1.05'])
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).toContain(`/api/v1/pairs/${pair}/limit-book/insert-hints?`)
    expect(url).toContain('side=bid')
    expect(url).toContain('prices=0.95%2C1.05')
  })

  it('URL-encodes pair and trader path segments (GitLab #379)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({}), { status: 200 }))
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          address: 'terra1trader000000000000000000000000000000',
          total_volume: '0',
          total_trades: 0,
        }),
        { status: 200 }
      )
    )
    const client = await loadModule()
    const pairWithSlash = 'terra1pair/with/slash'
    const traderWithSlash = 'terra1trader/with/slash'
    await client.getPair(pairWithSlash)
    await client.getTrader(traderWithSlash)
    expect(vi.mocked(fetch).mock.calls[0][0]).toBe(
      `${client.INDEXER_URL}/api/v1/pairs/${encodeURIComponent(pairWithSlash)}`
    )
    expect(vi.mocked(fetch).mock.calls[1][0]).toBe(
      `${client.INDEXER_URL}/api/v1/traders/${encodeURIComponent(traderWithSlash)}`
    )
  })
})

describe('indexer client fetchTraderHistoryCsv (GitLab #479)', () => {
  it('requests format=csv with clamped limit and returns text', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response('id,offer_amount\n1,100\n', { status: 200 }))
    const client = await loadModule()
    const addr = 'terra1trader000000000000000000000000000000'
    const pair = 'terra1pair0000000000000000000000000000000001'
    const csv = await client.fetchTraderHistoryCsv('trades', addr, { pair, limit: 500 })
    expect(csv).toContain('offer_amount')
    const url = vi.mocked(fetch).mock.calls[0][0] as string
    expect(url).toContain(`/api/v1/traders/${addr}/trades?`)
    expect(url).toContain('format=csv')
    expect(url).toContain(`limit=${client.TRADER_HISTORY_CSV_MAX_LIMIT}`)
    expect(url).toContain(`pair=${encodeURIComponent(pair)}`)
  })

  it('retries once on network failure then succeeds', async () => {
    vi.mocked(fetch).mockRejectedValueOnce(new Error('Failed to fetch'))
    vi.mocked(fetch).mockResolvedValueOnce(new Response('id\n1\n', { status: 200 }))
    const client = await loadModule()
    const csv = await client.fetchTraderHistoryCsv('limit-fills', 'terra1abc', { pair: 'terra1pair' })
    expect(csv).toBe('id\n1\n')
    expect(fetch).toHaveBeenCalledTimes(2)
  })

  it('throws on non-ok without retrying HTTP errors', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(new Response('nope', { status: 503, statusText: 'Service Unavailable' }))
    const client = await loadModule()
    await expect(client.fetchTraderHistoryCsv('limit-cancellations', 'terra1abc')).rejects.toThrow(
      'Indexer API error: 503'
    )
    expect(fetch).toHaveBeenCalledTimes(1)
  })
})

describe('indexer route/solve timeouts and AbortSignal (#484)', () => {
  it('exports a longer route-solve timeout than the default indexer fetch timeout', async () => {
    const client = await loadModule()
    expect(client.INDEXER_ROUTE_SOLVE_TIMEOUT_MS).toBeGreaterThan(client.INDEXER_FETCH_TIMEOUT_MS)
  })

  it('aborts getRouteSolve when the caller signal aborts without retrying', async () => {
    const ac = new AbortController()
    vi.mocked(fetch).mockImplementation(
      (_url, init) =>
        new Promise((_resolve, reject) => {
          init?.signal?.addEventListener('abort', () => {
            reject(new DOMException('The operation was aborted.', 'AbortError'))
          })
        })
    )
    const client = await loadModule()
    const pending = client.getRouteSolve('terra1a', 'terra1b', '1000', { signal: ac.signal })
    ac.abort()
    await expect(pending).rejects.toThrow(/abort/i)
    expect(fetch).toHaveBeenCalledTimes(1)
  })

  it('getRouteSolveProgress polls progress endpoint with amount_in (#485)', async () => {
    const body = {
      stage: 'evaluating',
      done: 2,
      total: 5,
      label: 'Searching 2 of 5 paths…',
      cache_hit: false,
      updated_at_ms: 1,
    }
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify(body), { status: 200 }))
    const client = await loadModule()
    const result = await client.getRouteSolveProgress('terra1a', 'terra1b', '1000', {
      maxMakerFills: 8,
      trader: 'terra1trader',
    })
    expect(result).toEqual(body)
    expect(fetch).toHaveBeenCalledWith(
      expect.stringMatching(/\/api\/v1\/route\/solve\/progress\?/),
      expect.objectContaining({ signal: expect.any(AbortSignal) })
    )
    const url = String(vi.mocked(fetch).mock.calls[0][0])
    expect(url).toContain('token_in=terra1a')
    expect(url).toContain('token_out=terra1b')
    expect(url).toContain('amount_in=1000')
    expect(url).toContain('max_maker_fills=8')
    expect(url).toContain('trader=terra1trader')
  })
})

describe('oracle ticker allowlist (GitLab #550)', () => {
  it('getOraclePrice allowlists ticker path segments', async () => {
    vi.mocked(fetch).mockImplementation(
      async () => new Response(JSON.stringify({ ticker: 'ustc', price_usd: '0.005', sources: [] }), { status: 200 })
    )
    const client = await loadModule()
    await client.getOraclePrice('lunc')
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/oracle/price/lunc')
    await client.getOraclePrice('vfdusd')
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/oracle/price/vfdusd')
    await client.getOraclePrice('../ustc')
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/oracle/price/ustc')
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).not.toContain('..')
    await client.getOraclePrice('javascript:alert(1)')
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/oracle/price/ustc')
    await client.getOraclePrice('fdusd')
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/oracle/price/ustc')
  })

  it('getOracleHistory allowlists ticker path segments', async () => {
    vi.mocked(fetch).mockImplementation(
      async () => new Response(JSON.stringify({ ticker: 'ustc', prices: [] }), { status: 200 })
    )
    const client = await loadModule()
    await client.getOracleHistory({ ticker: 'vfdusd', limit: 48 })
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/oracle/history/vfdusd')
    await client.getOracleHistory({ ticker: '<img src=x>', limit: 48 })
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/oracle/history/ustc')
  })
})

describe('hub price ticker allowlist (GitLab #556)', () => {
  it('getHubPrice allowlists ticker path segments and skips injection', async () => {
    vi.mocked(fetch).mockImplementation(
      async () => new Response(JSON.stringify({ ticker: 'ustr', price_usd: '0.01' }), { status: 200 })
    )
    const client = await loadModule()
    await client.getHubPrice('ustr')
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).toContain('/hub-prices/ustr')
    const skipped = await client.getHubPrice('../ustr')
    expect(skipped).toBeNull()
    expect(String(vi.mocked(fetch).mock.calls.at(-1)?.[0])).not.toContain('..')
    const js = await client.getHubPrice('javascript:alert(1)')
    expect(js).toBeNull()
  })
})
