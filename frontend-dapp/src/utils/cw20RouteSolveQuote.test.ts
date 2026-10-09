import { getRouteSolve, getRouteSolveExactOutput } from '@/services/indexer/client'
import { queryContract } from '@/services/terraclassic/queries'
import type { IndexerRouteSolveResponse } from '@/types'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { quoteCw20ExactOutputViaRouteSolve, quoteCw20ViaRouteSolve } from './cw20RouteSolveQuote'
import { assertLocalRouteSubmit } from './localRouteQuote'

vi.mock('@/services/indexer/client', () => ({ getRouteSolve: vi.fn(), getRouteSolveExactOutput: vi.fn() }))
vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(() => {
    throw new Error('RPC forbidden')
  }),
}))
const from = 'terra1from'
const mid = 'terra1middle'
const to = 'terra1to'
const trader = 'terra1trader'
const input = {
  fromToken: from,
  toToken: to,
  simRaw: '1000000',
  maxMakerFills: 8,
  slippageTolerancePercent: 5,
  maxSpreadStr: '0.05',
  quoteTrader: { trader },
}
function fixture(): IndexerRouteSolveResponse {
  const hops = [
    { pair: 'pair1', offer_token: from, ask_token: mid },
    { pair: 'pair2', offer_token: mid, ask_token: to },
  ]
  const executionHops = hops.map((h, i) => ({
    ...h,
    offer_amount: i ? '990000' : '1000000',
    return_amount: i ? '980000' : '990000',
    pool_return_amount: i ? '980000' : '990000',
    book_return_net: '0',
    pool_input_actual: i ? '990000' : '1000000',
    book_offer_consumed: '0',
    spread_amount: '1000',
    commission_amount: '1000',
    min_return: i ? '931000' : '940500',
  }))
  return {
    token_in: from,
    token_out: to,
    hops,
    intermediate_tokens: [from, mid, to],
    router_operations: hops.map((h, i) => ({
      terra_swap: {
        offer_asset_info: { token: { contract_addr: h.offer_token } },
        ask_asset_info: { token: { contract_addr: h.ask_token } },
        min_return: executionHops[i].min_return,
      },
    })),
    estimated_amount_out: '980000',
    spot_amount_out: '1000000',
    quote_kind: 'indexer_pool_db',
    execution_metadata: {
      version: 1,
      generation: 'market-7',
      input_amount: '1000000',
      max_spread: '0.05',
      trader,
      observed_at_ms: Date.now() - 1000,
      expires_at_ms: Date.now() + 10000,
      policy: 'pool_only',
      minimum_receive: '931000',
      hops: executionHops,
    },
  }
}
function serve(idx = fixture()) {
  vi.mocked(getRouteSolve).mockResolvedValue(idx)
  return idx
}

beforeEach(() => vi.clearAllMocks())
describe('local executable route #1452', () => {
  it('quotes, refreshes and prepares reviewed operations without any chain request', async () => {
    serve()
    const first = await quoteCw20ViaRouteSolve(input)
    const refresh = await quoteCw20ViaRouteSolve(input)
    expect(first).toEqual(refresh)
    expect(first).toMatchObject({
      return_amount: '980000',
      routeSlippagePercent: '2.00',
      executionMetadata: { generation: 'market-7' },
      routePreflight: { anyHopExceedsMaxSpread: false },
    })
    expect(first!.indexerOperations.map((op) => op.terra_swap.min_return)).toEqual(['940500', '931000'])
    assertLocalRouteSubmit(
      first!.executionMetadata,
      first!.indexerOperations,
      input.simRaw,
      input.maxSpreadStr,
      trader,
      '931000'
    )
    expect(getRouteSolve).toHaveBeenCalledWith(
      from,
      to,
      input.simRaw,
      expect.objectContaining({ maxSpread: '0.05', trader })
    )
    expect(queryContract).not.toHaveBeenCalled()
  })
  it.each(['execution_metadata', 'estimated_amount_out'] as const)(
    'rejects missing %s without live fallback',
    async (key) => {
      const idx = fixture()
      delete idx[key]
      serve(idx)
      await expect(quoteCw20ViaRouteSolve(input)).rejects.toThrow('Quote unavailable')
      expect(queryContract).not.toHaveBeenCalled()
    }
  )
  it.each(['input_amount', 'max_spread', 'trader', 'sender', 'generation'] as const)(
    'rejects another %s identity',
    async (key) => {
      const idx = fixture()
      idx.execution_metadata![key] = key === 'generation' ? '' : 'wrong'
      serve(idx)
      await expect(quoteCw20ViaRouteSolve(input)).rejects.toThrow('Quote unavailable')
    }
  )
  it.each([0, 1, 2])('blocks unsupported hybrid, weak floor and expired observations (%s)', async (mode) => {
    const idx = fixture()
    if (mode === 0)
      (idx.router_operations[0] as { terra_swap: Record<string, unknown> }).terra_swap.hybrid = {
        pool_input: '1000000',
        book_input: '1',
      }
    if (mode === 1) {
      idx.execution_metadata!.hops[0].min_return = '1'
      ;(idx.router_operations[0] as { terra_swap: Record<string, unknown> }).terra_swap.min_return = '1'
    }
    if (mode === 2) idx.execution_metadata!.expires_at_ms = Date.now() - 1
    serve(idx)
    await expect(quoteCw20ViaRouteSolve(input)).rejects.toThrow('Quote unavailable')
  })
  it('uses contract L9 denominator rather than adding spread to gross output', async () => {
    const idx = fixture()
    idx.execution_metadata!.hops[0].spread_amount = '51000'
    serve(idx)
    await expect(quoteCw20ViaRouteSolve(input)).rejects.toThrow('Quote unavailable')
  })
  it('rejects corrupted propagation or operations and does not turn unknown reference into zero', async () => {
    const idx = fixture()
    delete idx.spot_amount_out
    delete idx.slippage_percent
    serve(idx)
    await expect(quoteCw20ViaRouteSolve(input)).rejects.toThrow('Quote unavailable')
    idx.spot_amount_out = '1000000'
    idx.execution_metadata!.hops[1].offer_amount = '990001'
    serve(idx)
    await expect(quoteCw20ViaRouteSolve(input)).rejects.toThrow('Quote unavailable')
  })
  it('rejects a forged signing path, other actor, changed amount or expired review', async () => {
    serve()
    const q = (await quoteCw20ViaRouteSolve(input))!
    const assert = (ops = q.indexerOperations, amount = input.simRaw, wallet = trader, now = Date.now()) =>
      assertLocalRouteSubmit(q.executionMetadata, ops, amount, input.maxSpreadStr, wallet, '931000', now)
    const forged = structuredClone(q.indexerOperations)
    forged[0].terra_swap.ask_asset_info = { token: { contract_addr: 'terra1attacker' } }
    expect(() => assert(forged)).toThrow('Quote unavailable')
    expect(() => assert(q.indexerOperations, '1000001')).toThrow('Quote unavailable')
    expect(() => assert(q.indexerOperations, input.simRaw, 'another')).toThrow('Quote unavailable')
    expect(() => assert(q.indexerOperations, input.simRaw, trader, q.executionMetadata.expires_at_ms)).toThrow(
      'Quote unavailable'
    )
  })
  it.each([
    [0.14, '0.0014'],
    [0.29, '0.0029'],
  ] as const)('preserves the actual custom %s%% constraint through quote validation', async (percent, spread) => {
    const idx = fixture()
    const atomics = BigInt(Math.round(percent * 100))
    idx.execution_metadata!.max_spread = spread
    for (const [i, hop] of idx.execution_metadata!.hops.entries()) {
      hop.min_return = ((BigInt(hop.return_amount) * (10000n - atomics)) / 10000n).toString()
      ;(idx.router_operations[i] as { terra_swap: { min_return: string } }).terra_swap.min_return = hop.min_return
    }
    idx.execution_metadata!.minimum_receive = idx.execution_metadata!.hops.at(-1)!.min_return
    serve(idx)
    const q = await quoteCw20ViaRouteSolve({ ...input, maxSpreadStr: spread, slippageTolerancePercent: percent })
    expect(q?.minimumReceive).toBe(idx.execution_metadata!.minimum_receive)
    expect(getRouteSolve).toHaveBeenCalledWith(from, to, input.simRaw, expect.objectContaining({ maxSpread: spread }))
  })
  it('cancellation stops validation and never discovers or simulates a fallback', async () => {
    serve()
    const controller = new AbortController()
    controller.abort()
    await expect(quoteCw20ViaRouteSolve({ ...input, signal: controller.signal })).rejects.toThrow()
    expect(queryContract).not.toHaveBeenCalled()
  })
  it('sizes exact net output from the same local snapshot with the selected spread', async () => {
    const idx = fixture()
    idx.sizing_mode = 'pool_only_exact_output'
    idx.required_amount_in = '1000000'
    idx.target_amount_out = '980000'
    vi.mocked(getRouteSolveExactOutput).mockResolvedValue(idx)
    const q = await quoteCw20ExactOutputViaRouteSolve({ ...input, targetRaw: '980000' })
    expect(q.minimumReceive).toBe('980000')
    expect(q.offerAmount).toBe('1000000')
    expect(q.return_amount).toBe('980000')
    expect(getRouteSolveExactOutput).toHaveBeenCalledWith(
      from,
      to,
      '980000',
      expect.objectContaining({ maxSpread: '0.05' })
    )
    expect(queryContract).not.toHaveBeenCalled()
  })
})
