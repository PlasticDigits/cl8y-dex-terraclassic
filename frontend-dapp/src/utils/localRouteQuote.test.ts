import { getRouteSolve } from '@/services/indexer/client'
import { queryContract } from '@/services/terraclassic/queries'
import { executeLocalNativeRoute, executeMultiHopSwap, type SwapOperation } from '@/services/terraclassic/router'
import { executeTerraContract, executeTerraContractMulti } from '@/services/terraclassic/transactions'
import { LOCAL_FROM, LOCAL_TO, LOCAL_WALLET, localQuote } from '@/test/localRouteFixtures'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { quoteCw20ViaRouteSolve } from './cw20RouteSolveQuote'
import { assertLocalRouteSubmit, validateLocalRouteQuote } from './localRouteQuote'
vi.mock('@/services/indexer/client', () => ({ getRouteSolve: vi.fn() }))
vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(() => {
    throw new Error('RPC forbidden')
  }),
}))
vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn().mockResolvedValue('tx'),
  executeTerraContractMulti: vi.fn().mockResolvedValue('tx'),
}))
vi.mock('@/utils/constants', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/utils/constants')>()),
  TREASURY_CONTRACT_ADDRESS: 'terra1treasury',
  WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1mapper',
  ROUTER_CONTRACT_ADDRESS: 'terra1router',
  NATIVE_WRAPPED_PAIRS: { uluna: 'terra1wrapped', uusd: 'terra1wrappedusd' },
}))
beforeEach(() => vi.clearAllMocks())
const request = {
  fromToken: LOCAL_FROM,
  toToken: LOCAL_TO,
  simRaw: '1000000',
  maxMakerFills: 8,
  slippageTolerancePercent: 5,
  maxSpreadStr: '0.05',
  quoteTrader: { trader: LOCAL_WALLET },
}
function nativeQuote(unwrap = false) {
  const q = localQuote(
    unwrap ? LOCAL_FROM : 'terra1wrapped',
    unwrap ? 'terra1wrappedusd' : LOCAL_TO,
    unwrap ? '1000000' : '990000',
    { trader: LOCAL_WALLET },
    '1000000'
  )
  q.token_in = unwrap ? LOCAL_FROM : 'uluna'
  q.token_out = unwrap ? 'uusd' : LOCAL_TO
  q.execution_metadata!.input_amount = '1000000'
  q.execution_metadata!.wrapper = {
    input_token: q.token_in,
    output_token: q.token_out,
    input_amount: '1000000',
    route_input_amount: unwrap ? '1000000' : '990000',
    route_output_amount: '1000000',
    minimum_receive_base: unwrap ? '940500' : '950000',
    wrap_input: !unwrap,
    unwrap_output: unwrap,
    treasury: 'terra1treasury',
    mapper: 'terra1mapper',
  }
  if (unwrap) {
    q.estimated_amount_out = '980100'
    q.execution_metadata!.minimum_receive = '931095'
    q.spot_amount_out = '980100'
  }
  return q
}
describe('local quote settlement policies #1452', () => {
  it('validates post-hook floors while L9 uses pre-hook pool output', () => {
    const q = localQuote()
    q.execution_metadata!.hops[0].pool_return_amount = '1050000'
    q.execution_metadata!.hops[0].commission_amount = '1000'
    expect(
      validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
        simRaw: '1000000',
        maxSpreadStr: '0.05',
        slippageTolerancePercent: 5,
      })
    ).toBeDefined()
  })
  it('retains a bounded hybrid hop0 plan with both per-hop and final net floors', async () => {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
    const h = q.execution_metadata!.hops[0]
    h.pool_return_amount = '800000'
    h.book_return_net = '200000'
    h.pool_input_actual = '800000'
    h.book_offer_consumed = '200000'
    q.execution_metadata!.policy = 'declared_hybrid_hop0'
    ;(q.router_operations[0] as SwapOperation).terra_swap.hybrid = {
      pool_input: '800000',
      book_input: '200000',
      max_maker_fills: 8,
      book_start_hint: null,
    }
    vi.mocked(getRouteSolve).mockResolvedValue(q)
    const result = (await quoteCw20ViaRouteSolve(request))!
    expect(result.indexerOperations[0].terra_swap.hybrid?.book_input).toBe('200000')
    await executeMultiHopSwap(
      LOCAL_WALLET,
      LOCAL_FROM,
      '1000000',
      result.indexerOperations,
      '0.05',
      '950000',
      undefined,
      undefined,
      result.executionMetadata
    )
    expect(queryContract).not.toHaveBeenCalled()
    const hook = JSON.parse(
      atob((vi.mocked(executeTerraContract).mock.calls[0][2] as { send: { msg: string } }).send.msg)
    )
    expect(hook.execute_swap_operations.operations[0].terra_swap.min_return).toBe('950000')
  })
  it('rejects book degradation, undersized pool legs and excessive maker fills', () => {
    for (const mode of [0, 1, 2]) {
      const q = localQuote()
      const h = q.execution_metadata!.hops[0]
      q.execution_metadata!.policy = 'declared_hybrid_hop0'
      const hybrid = { pool_input: '800000', book_input: '200000', max_maker_fills: 8, book_start_hint: null }
      ;(q.router_operations[0] as SwapOperation).terra_swap.hybrid = hybrid
      h.pool_return_amount = '800000'
      h.book_return_net = mode === 0 ? '1' : '200000'
      h.pool_input_actual = '800000'
      h.book_offer_consumed = '200000'
      if (mode === 1) {
        hybrid.pool_input = '1'
        hybrid.book_input = '999999'
      }
      if (mode === 2) hybrid.max_maker_fills = 9
      expect(() =>
        validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
          simRaw: '1000000',
          maxSpreadStr: '0.05',
          slippageTolerancePercent: 5,
          maxMakerFills: 8,
        })
      ).toThrow('Quote unavailable')
    }
  })
  it('quotes and signs native input using the exact gross deposit and cached post-fee router send', async () => {
    vi.mocked(getRouteSolve).mockResolvedValue(nativeQuote())
    const q = (await quoteCw20ViaRouteSolve({ ...request, fromToken: 'uluna' }))!
    await executeLocalNativeRoute(LOCAL_WALLET, q.executionMetadata, q.indexerOperations, '0.05', '950000', 123)
    expect(vi.mocked(executeTerraContractMulti).mock.calls[0][1]).toEqual([
      { contract: 'terra1treasury', msg: { wrap_deposit: {} }, coins: [{ denom: 'uluna', amount: '1000000' }] },
      {
        contract: 'terra1wrapped',
        msg: expect.objectContaining({ send: expect.objectContaining({ contract: 'terra1router', amount: '990000' }) }),
      },
    ])
    expect(queryContract).not.toHaveBeenCalled()
  })
  it('quotes and signs native output with the cached router unwrap floor', async () => {
    vi.mocked(getRouteSolve).mockResolvedValue(nativeQuote(true))
    const q = (await quoteCw20ViaRouteSolve({ ...request, toToken: 'uusd' }))!
    expect(q.return_amount).toBe('980100')
    await executeLocalNativeRoute(LOCAL_WALLET, q.executionMetadata, q.indexerOperations, '0.05', '940500')
    const send = vi.mocked(executeTerraContract).mock.calls[0][2] as { send: { msg: string } }
    expect(JSON.parse(atob(send.send.msg)).execute_swap_operations).toMatchObject({
      unwrap_output: true,
      minimum_receive: '940500',
    })
    expect(queryContract).not.toHaveBeenCalled()
  })
  it('rejects wrong native gross amount, mapper identity and wrapped-token mapping', async () => {
    for (const mode of [0, 1, 2]) {
      const q = nativeQuote()
      const w = q.execution_metadata!.wrapper!
      if (mode === 0) w.input_amount = '999999'
      if (mode === 1) w.mapper = 'terra1attacker'
      if (mode === 2) q.hops[0].offer_token = LOCAL_FROM
      vi.mocked(getRouteSolve).mockResolvedValue(q)
      await expect(quoteCw20ViaRouteSolve({ ...request, fromToken: 'uluna' })).rejects.toThrow()
    }
    expect(queryContract).not.toHaveBeenCalled()
  })
  it('pins the minimum receive and cannot sign a weaker floor after a refresh', () => {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
    expect(() =>
      assertLocalRouteSubmit(
        q.execution_metadata!,
        q.router_operations as SwapOperation[],
        '1000000',
        '0.05',
        LOCAL_WALLET,
        '949999'
      )
    ).toThrow('Quote unavailable')
  })
})
