import { getRouteSolve, postRouteSolve } from '@/services/indexer/client'
import { queryContract } from '@/services/terraclassic/queries'
import { executeLocalNativeRoute, executeMultiHopSwap, type SwapOperation } from '@/services/terraclassic/router'
import { executeTerraContract, executeTerraContractMulti } from '@/services/terraclassic/transactions'
import { LOCAL_FROM, LOCAL_TO, LOCAL_WALLET, localQuote, localTaxQuote } from '@/test/localRouteFixtures'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { quoteCw20ViaRouteSolve } from './cw20RouteSolveQuote'
import { assertLocalRouteSubmit, validateLocalRouteQuote } from './localRouteQuote'
import { buildSubmitAlignedSimPayload } from '@/utils/quoteDebounce'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
vi.mock('@/services/indexer/client', () => ({ getRouteSolve: vi.fn(), postRouteSolve: vi.fn() }))
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
  it.each([
    [30000, 10000],
    [15000, 5001],
    [1000, Number.NaN],
    [1000, Number.POSITIVE_INFINITY],
  ])('rejects renewed/unsafe original windows: age%s expiry%s (#1473)', async (age, expiry) => {
    const now = Date.now()
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
    q.execution_metadata!.observed_at_ms = now - age
    q.execution_metadata!.expires_at_ms = now + expiry
    expect(() =>
      validateLocalRouteQuote(q, q.router_operations as SwapOperation[], { ...request, trader: LOCAL_WALLET }, now)
    ).toThrow('Quote unavailable')
    expect(() =>
      assertLocalRouteSubmit(
        q.execution_metadata!,
        q.router_operations as SwapOperation[],
        '1000000',
        '0.05',
        LOCAL_WALLET,
        '950000',
        now
      )
    ).toThrow('Quote unavailable')
    vi.mocked(getRouteSolve).mockResolvedValue(q)
    await expect(quoteCw20ViaRouteSolve(request)).rejects.toThrow('Quote unavailable')
    expect(queryContract).not.toHaveBeenCalled()
  })
  it('retains an unchanged original20s expiry and rejects exactly at expiration (#1473)', () => {
    const now = Date.now()
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
    q.execution_metadata!.observed_at_ms = now - 1000
    q.execution_metadata!.expires_at_ms = now + 19000
    expect(
      validateLocalRouteQuote(q, q.router_operations as SwapOperation[], { ...request, trader: LOCAL_WALLET }, now)
        .metadata.expires_at_ms
    ).toBe(now + 19000)
    assertLocalRouteSubmit(
      q.execution_metadata!,
      q.router_operations as SwapOperation[],
      '1000000',
      '0.05',
      LOCAL_WALLET,
      '950000',
      now
    )
    expect(() =>
      assertLocalRouteSubmit(
        q.execution_metadata!,
        q.router_operations as SwapOperation[],
        '1000000',
        '0.05',
        LOCAL_WALLET,
        '950000',
        now + 19000
      )
    ).toThrow('Quote unavailable')
  })
  it.each(['false', 0, 1, null, {}])(
    'rejects malformed execution capability %j before validation or signing (#1473)',
    async (flag) => {
      const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
      q.execution_metadata!.execution_supported = flag as boolean
      expect(() =>
        validateLocalRouteQuote(q, q.router_operations as SwapOperation[], { ...request, trader: LOCAL_WALLET })
      ).toThrow('Quote unavailable')
      expect(() =>
        assertLocalRouteSubmit(
          q.execution_metadata!,
          q.router_operations as SwapOperation[],
          '1000000',
          '0.05',
          LOCAL_WALLET,
          '950000'
        )
      ).toThrow('Quote unavailable')
      vi.mocked(getRouteSolve).mockResolvedValue(q)
      await expect(quoteCw20ViaRouteSolve(request)).rejects.toThrow('Quote unavailable')
      delete q.execution_metadata!.execution_supported
      q.execution_supported = flag as boolean
      expect(() =>
        validateLocalRouteQuote(q, q.router_operations as SwapOperation[], { ...request, trader: LOCAL_WALLET })
      ).toThrow('Quote unavailable')
      await expect(quoteCw20ViaRouteSolve(request)).rejects.toThrow('Quote unavailable')
      expect(queryContract).not.toHaveBeenCalled()
    }
  )
  it('accepts reviewed Boolean support and rejects contradictory top-level claims (#1473)', () => {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
    q.execution_metadata!.execution_supported = true
    q.execution_supported = true
    expect(
      validateLocalRouteQuote(q, q.router_operations as SwapOperation[], { ...request, trader: LOCAL_WALLET })
    ).toBeDefined()
    q.execution_supported = false
    expect(() =>
      validateLocalRouteQuote(q, q.router_operations as SwapOperation[], { ...request, trader: LOCAL_WALLET })
    ).toThrow('Quote unavailable')
  })
  it('retains explicit legacy validation but prevents official net19/min18 policy downgrades (#1473)', async () => {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '19', { trader: LOCAL_WALLET }, '19')
    delete q.execution_metadata!.protection_policy
    delete q.execution_metadata!.execution_market_spread
    q.execution_metadata!.minimum_receive = '18'
    q.execution_metadata!.hops[0].min_return = '18'
    ;(q.router_operations[0] as SwapOperation).terra_swap.min_return = '18'
    expect(
      validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
        ...request,
        simRaw: '19',
        trader: LOCAL_WALLET,
      })
    ).toBeDefined()
    vi.mocked(getRouteSolve).mockResolvedValue(q)
    await expect(quoteCw20ViaRouteSolve({ ...request, simRaw: '19' })).rejects.toThrow('Quote unavailable')
    expect(queryContract).not.toHaveBeenCalled()
  })
  it.each([
    ['2', '2'],
    ['19', '19'],
    ['20', '19'],
  ])('keeps selected tolerance at raw output%s and floor%s (#1473)', (output, floor) => {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, output, { trader: LOCAL_WALLET }, output)
    const metadata = q.execution_metadata!
    metadata.protection_policy = 'quote_relative_v1'
    metadata.execution_market_spread = '0.052631578947368422'
    metadata.minimum_receive = floor
    metadata.hops[0].min_return = floor
    ;(q.router_operations[0] as SwapOperation).terra_swap.min_return = floor
    const validated = validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
      ...request,
      simRaw: output,
      trader: LOCAL_WALLET,
    })
    const reviewed = buildSubmitAlignedSimPayload(
      output,
      { return_amount: output, minimumReceive: validated.metadata.minimum_receive },
      5,
      applySlippagePercentFloor
    )
    expect(reviewed.minReceived).toBe(floor)
    expect((BigInt(output) - BigInt(floor)) * 100n <= BigInt(output) * 5n).toBe(true)
    assertLocalRouteSubmit(metadata, q.router_operations as SwapOperation[], output, '0.05', LOCAL_WALLET, floor)
    metadata.hops[0].min_return = (BigInt(floor) - 1n).toString()
    ;(q.router_operations[0] as SwapOperation).terra_swap.min_return = metadata.hops[0].min_return
    expect(() =>
      assertLocalRouteSubmit(metadata, q.router_operations as SwapOperation[], output, '0.05', LOCAL_WALLET, floor)
    ).toThrow('Quote unavailable')
  })
  it('executes a pinned high-impact baseline with exact UI net floors and an independently derived market ceiling (#1473)', async () => {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '10000', { trader: LOCAL_WALLET }, '9000')
    const metadata = q.execution_metadata!,
      hop = metadata.hops[0]
    metadata.protection_policy = 'quote_relative_v1'
    metadata.execution_market_spread = '0.157894736842105264'
    hop.commission_amount = '1000'
    hop.spread_amount = '1000'
    q.spot_amount_out = '10000'
    vi.mocked(getRouteSolve).mockResolvedValue(q)
    const result = (await quoteCw20ViaRouteSolve({ ...request, simRaw: '10000', trader: LOCAL_WALLET }))!
    expect(result.return_amount).toBe('9000')
    expect(result.minimumReceive).toBe('8550')
    expect(result.routePreflight?.anyHopExceedsMaxSpread).toBe(false)
    expect(result.routePreflight?.worstSpreadPercent).toBe('10.00')
    await executeMultiHopSwap(
      LOCAL_WALLET,
      LOCAL_FROM,
      '10000',
      result.indexerOperations,
      '0.05',
      '8550',
      undefined,
      undefined,
      result.executionMetadata
    )
    const hook = JSON.parse(
      atob((vi.mocked(executeTerraContract).mock.calls[0][2] as { send: { msg: string } }).send.msg)
    )
    expect(hook.execute_swap_operations.max_spread).toBe('0.157894736842105264')
    expect(hook.execute_swap_operations.minimum_receive).toBe('8550')
    expect(hook.execute_swap_operations.operations[0].terra_swap.min_return).toBe('8550')
    expect(metadata.max_spread).toBe('0.05')
    expect(queryContract).not.toHaveBeenCalled()
    const legacy = structuredClone(q)
    delete legacy.execution_metadata!.protection_policy
    delete legacy.execution_metadata!.execution_market_spread
    expect(() =>
      validateLocalRouteQuote(legacy, legacy.router_operations as SwapOperation[], {
        ...request,
        simRaw: '10000',
        trader: LOCAL_WALLET,
      })
    ).toThrow('Quote unavailable')
  })

  it.each(['0.05', '0.157894736842105263', '0.157894736842105265', '1', '1e-1'])(
    'rejects tampered quote-relative ceiling %s (#1473)',
    (ceiling) => {
      const q = localQuote(LOCAL_FROM, LOCAL_TO, '10000', { trader: LOCAL_WALLET }, '9000')
      q.execution_metadata!.hops[0].commission_amount = '1000'
      q.execution_metadata!.hops[0].spread_amount = '1000'
      q.execution_metadata!.protection_policy = 'quote_relative_v1'
      q.execution_metadata!.execution_market_spread = ceiling
      q.spot_amount_out = '10000'
      expect(() =>
        validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
          ...request,
          simRaw: '10000',
          trader: LOCAL_WALLET,
        })
      ).toThrow('Quote unavailable')
      expect(() =>
        assertLocalRouteSubmit(
          q.execution_metadata!,
          q.router_operations as SwapOperation[],
          '10000',
          '0.05',
          LOCAL_WALLET,
          '8550'
        )
      ).toThrow('Quote unavailable')
    }
  )

  it.each(['pool_return_amount', 'spread_amount', 'return_amount', 'min_return'] as const)(
    'rejects noncanonical quote-relative %s (#1473)',
    (field) => {
      const q = localQuote(LOCAL_FROM, LOCAL_TO, '10000', { trader: LOCAL_WALLET }, '9000')
      q.execution_metadata!.protection_policy = 'quote_relative_v1'
      q.execution_metadata!.execution_market_spread = '0.052631578947368422'
      q.execution_metadata!.hops[0][field] = `0${q.execution_metadata!.hops[0][field]}`
      if (field === 'min_return')
        (q.router_operations[0] as SwapOperation).terra_swap.min_return = q.execution_metadata!.hops[0][field]
      expect(() =>
        validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
          ...request,
          simRaw: '10000',
          trader: LOCAL_WALLET,
        })
      ).toThrow('Quote unavailable')
      expect(() =>
        assertLocalRouteSubmit(
          q.execution_metadata!,
          q.router_operations as SwapOperation[],
          '10000',
          '0.05',
          LOCAL_WALLET,
          '8550'
        )
      ).toThrow('Quote unavailable')
    }
  )

  it('rejects weakened simultaneous metadata/operation net floors at signing (#1473)', () => {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '10000', { trader: LOCAL_WALLET }, '9000')
    q.execution_metadata!.hops[0].commission_amount = '1000'
    q.execution_metadata!.hops[0].spread_amount = '1000'
    q.execution_metadata!.protection_policy = 'quote_relative_v1'
    q.execution_metadata!.execution_market_spread = '0.157894736842105264'
    q.execution_metadata!.hops[0].min_return = '8549'
    ;(q.router_operations[0] as SwapOperation).terra_swap.min_return = '8549'
    expect(() =>
      assertLocalRouteSubmit(
        q.execution_metadata!,
        q.router_operations as SwapOperation[],
        '10000',
        '0.05',
        LOCAL_WALLET,
        '8550'
      )
    ).toThrow('Quote unavailable')
  })
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
    h.declared_hybrid = structuredClone((q.router_operations[0] as SwapOperation).terra_swap.hybrid!)
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

describe('local gross funding v1 #1452', () => {
  function taxedQuote() {
    const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET }, '966700')
    q.tax_funding_version = 1
    q.tax_kind = 'route_local_v1'
    q.router_hops_tax = true
    q.sell_tax_bps = 250
    q.buy_tax_bps = 333
    q.execution_metadata!.tax_funding_version = 1
    const h = q.execution_metadata!.hops[0]
    Object.assign(h, {
      economic_offer_amount: '975610',
      sell_tax_amount: '24390',
      sell_residue: '0',
      sell_tax_bps: 250,
      buy_tax_bps: 333,
      buy_tax_amount: '33300',
      post_hook_return_amount: '1000000',
      pool_input_actual: '975610',
      pool_return_amount: '1000000',
    })
    return q
  }
  it('validates exact funded pair credit and signs only the authorized gross without request RPC', async () => {
    const q = taxedQuote()
    vi.mocked(getRouteSolve).mockResolvedValue(q)
    const result = (await quoteCw20ViaRouteSolve(request))!
    expect(result.return_amount).toBe('966700')
    await executeMultiHopSwap(
      LOCAL_WALLET,
      LOCAL_FROM,
      '1000000',
      result.indexerOperations,
      '0.05',
      result.executionMetadata.minimum_receive,
      undefined,
      undefined,
      result.executionMetadata
    )
    expect(queryContract).not.toHaveBeenCalled()
    const send = vi.mocked(executeTerraContract).mock.calls[0][2] as { send: { amount: string; msg: string } }
    expect(send.send.amount).toBe('1000000')
    const hook = JSON.parse(atob(send.send.msg))
    expect(hook.execute_route_swap_operations).toMatchObject({ funding_version: 1, minimum_receive: '918365' })
    expect(hook.execute_swap_operations).toBeUndefined()
  })
  it('rejects underfunded credit, nonmaximal sell, double buy tax and unpinned funding semantics', () => {
    for (const mutate of [
      (q: ReturnType<typeof taxedQuote>) => {
        q.execution_metadata!.hops[0].economic_offer_amount = '975609'
      },
      (q: ReturnType<typeof taxedQuote>) => {
        q.execution_metadata!.hops[0].sell_residue = '1'
      },
      (q: ReturnType<typeof taxedQuote>) => {
        q.execution_metadata!.hops[0].buy_tax_amount = '66600'
      },
      (q: ReturnType<typeof taxedQuote>) => {
        q.tax_funding_version = undefined
      },
      (q: ReturnType<typeof taxedQuote>) => {
        q.execution_metadata!.hops[0].pool_input_actual = '1000000'
      },
    ]) {
      const q = taxedQuote()
      mutate(q)
      expect(() =>
        validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
          simRaw: '1000000',
          maxSpreadStr: '0.05',
          slippageTolerancePercent: 5,
          trader: LOCAL_WALLET,
        })
      ).toThrow('Quote unavailable')
    }
  })
  it('validates hybrid economic budgets and rejects a gross split relabeled as funded credit', () => {
    const q = taxedQuote(),
      h = q.execution_metadata!.hops[0]
    q.execution_metadata!.policy = 'declared_hybrid_manual'
    h.pool_input_actual = '780488'
    h.book_offer_consumed = '195122'
    h.pool_return_amount = '800000'
    h.book_return_net = '200000'
    const hybrid = { pool_input: '780488', book_input: '195122', max_maker_fills: 8, book_start_hint: null }
    ;(q.router_operations[0] as SwapOperation).terra_swap.hybrid = hybrid
    h.declared_hybrid = structuredClone(hybrid)
    const input = { simRaw: '1000000', maxSpreadStr: '0.05', slippageTolerancePercent: 5, trader: LOCAL_WALLET }
    expect(validateLocalRouteQuote(q, q.router_operations as SwapOperation[], input)).toBeDefined()
    hybrid.pool_input = '800000'
    hybrid.book_input = '200000'
    expect(() => validateLocalRouteQuote(q, q.router_operations as SwapOperation[], input)).toThrow('Quote unavailable')
  })
})

it('keeps native gross deposit and per-hop v1 tax accounting in the reviewed signer', async () => {
  const q = localTaxQuote('terra1wrapped', LOCAL_TO, '990000', { trader: LOCAL_WALLET }, 0, 333)
  q.token_in = 'uluna'
  q.execution_metadata!.input_amount = '1000000'
  q.execution_metadata!.wrapper = {
    input_token: 'uluna',
    output_token: LOCAL_TO,
    input_amount: '1000000',
    route_input_amount: '990000',
    route_output_amount: '966700',
    minimum_receive_base: '918365',
    wrap_input: true,
    unwrap_output: false,
    treasury: 'terra1treasury',
    mapper: 'terra1mapper',
  }
  vi.mocked(getRouteSolve).mockResolvedValue(q)
  const result = (await quoteCw20ViaRouteSolve({ ...request, fromToken: 'uluna' }))!
  expect(result.taxFundingVersion).toBe(1)
  await executeLocalNativeRoute(LOCAL_WALLET, result.executionMetadata, result.indexerOperations, '0.05', '918365')
  expect(queryContract).not.toHaveBeenCalled()
  const messages = vi.mocked(executeTerraContractMulti).mock.calls[0][1]
  expect(messages[0].coins).toEqual([{ denom: 'uluna', amount: '1000000' }])
  const send = messages[1].msg as { send: { amount: string; msg: string } }
  const hook = JSON.parse(atob(send.send.msg))
  expect(send.send.amount).toBe('990000')
  expect(hook.execute_route_swap_operations).toMatchObject({ funding_version: 1, minimum_receive: '918365' })
  expect(hook.execute_swap_operations).toBeUndefined()
})

it('pins gross manual ratios to exact funded economic splits, including zero book budgets', async () => {
  for (const book of ['200000', '0']) {
    const q = localTaxQuote(LOCAL_FROM, LOCAL_TO, '1050000', { trader: LOCAL_WALLET })
    const h = q.execution_metadata!.hops[0],
      economicBook = BigInt(book)
    q.execution_metadata!.policy = 'declared_hybrid_manual'
    h.pool_input_actual = (1000000n - economicBook).toString()
    h.book_offer_consumed = book
    h.pool_return_amount = h.pool_input_actual
    h.book_return_net = book
    ;(q.router_operations[0] as SwapOperation).terra_swap.hybrid = {
      pool_input: h.pool_input_actual,
      book_input: book,
      max_maker_fills: 8,
      book_start_hint: null,
    }
    h.declared_hybrid = structuredClone((q.router_operations[0] as SwapOperation).terra_swap.hybrid!)
    const declaredBook = (economicBook * 1050000n) / 1000000n
    const declared = {
      pool_input: (1050000n - declaredBook).toString(),
      book_input: declaredBook.toString(),
      max_maker_fills: 8,
    }
    vi.mocked(postRouteSolve).mockResolvedValue(q)
    const result = (await quoteCw20ViaRouteSolve({ ...request, simRaw: '1050000', declaredHybrid: [declared] }))!
    expect(result.indexerOperations[0].terra_swap.hybrid?.book_input).toBe(book)
    expect(result.indexerOperations[0].terra_swap.hybrid?.pool_input).toBe(h.pool_input_actual)
    await executeMultiHopSwap(
      LOCAL_WALLET,
      LOCAL_FROM,
      '1050000',
      result.indexerOperations,
      '0.05',
      '950000',
      undefined,
      undefined,
      result.executionMetadata
    )
    const send = vi.mocked(executeTerraContract).mock.calls.at(-1)![2] as { send: { amount: string; msg: string } }
    expect(send.send.amount).toBe('1050000')
    const hook = JSON.parse(atob(send.send.msg))
    expect(hook.execute_route_swap_operations).toMatchObject({ funding_version: 1, minimum_receive: '950000' })
    expect(hook.execute_route_swap_operations.operations[0].terra_swap.hybrid).toMatchObject({
      pool_input: h.pool_input_actual,
      book_input: book,
      max_maker_fills: 8,
    })
    expect(hook.execute_swap_operations).toBeUndefined()
  }
  expect(queryContract).not.toHaveBeenCalled()
})

it('rejects changing reviewed hybrid budgets or maker cap before mutation', async () => {
  const q = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
  const h = q.execution_metadata!.hops[0]
  q.execution_metadata!.policy = 'declared_hybrid_manual'
  h.pool_input_actual = '800000'
  h.book_offer_consumed = '200000'
  h.pool_return_amount = '800000'
  h.book_return_net = '200000'
  const hybrid = { pool_input: '800000', book_input: '200000', max_maker_fills: 8, book_start_hint: null }
  h.declared_hybrid = structuredClone(hybrid)
  ;(q.router_operations[0] as SwapOperation).terra_swap.hybrid = hybrid
  validateLocalRouteQuote(q, q.router_operations as SwapOperation[], {
    simRaw: '1000000',
    maxSpreadStr: '0.05',
    slippageTolerancePercent: 5,
    trader: LOCAL_WALLET,
  })
  for (const mutate of [
    (h: typeof hybrid) => {
      h.pool_input = '700000'
      h.book_input = '300000'
    },
    (h: typeof hybrid) => {
      h.max_maker_fills = 9
    },
  ]) {
    const ops = structuredClone(q.router_operations) as SwapOperation[]
    mutate(ops[0].terra_swap.hybrid! as typeof hybrid)
    await expect(
      executeMultiHopSwap(
        LOCAL_WALLET,
        LOCAL_FROM,
        '1000000',
        ops,
        '0.05',
        '950000',
        undefined,
        undefined,
        q.execution_metadata
      )
    ).rejects.toThrow('Quote unavailable')
  }
  expect(executeTerraContract).not.toHaveBeenCalled()
})

it('rejects changing the observed final wallet recipient on a local v1 route', async () => {
  const q = localTaxQuote(LOCAL_FROM, LOCAL_TO, '1050000', { trader: LOCAL_WALLET })
  await expect(
    executeMultiHopSwap(
      LOCAL_WALLET,
      LOCAL_FROM,
      '1050000',
      q.router_operations as SwapOperation[],
      '0.05',
      '950000',
      'terra1unobservedrecipient',
      undefined,
      q.execution_metadata
    )
  ).rejects.toThrow('Quote unavailable')
  expect(executeTerraContract).not.toHaveBeenCalled()
  expect(queryContract).not.toHaveBeenCalled()
})
