import { beforeEach, describe, expect, it, vi } from 'vitest'
import { getRouteSolve, getRouteSolveExactOutput } from '@/services/indexer/client'
import { queryContract } from '@/services/terraclassic/queries'
import { executeTerraContract, executeTerraContractMulti } from '@/services/terraclassic/transactions'
import { executeLocalNativeRoute } from '@/services/terraclassic/router'
import type { IndexerRouteSolveResponse } from '@/types'
import { quoteCw20ExactOutputViaRouteSolve, quoteCw20ViaRouteSolve } from './cw20RouteSolveQuote'
import { assertLocalRouteSubmit } from './localRouteQuote'
import { DIRECT_WRAPPER_FLOOR_UNSUPPORTED, validateWrapperOnlyQuote } from './localWrapperOnlyQuote'
import { evaluateTradingRisk, tradingRiskBlocksSubmit } from './tradingRisk'
import { LOCAL_QUOTE_SIGNING_WINDOW_MS } from '@/utils/localQuoteWindow'
vi.mock('@/services/indexer/client', () => ({
  getRouteSolve: vi.fn(),
  getRouteSolveExactOutput: vi.fn(),
  postRouteSolve: vi.fn(),
}))
vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(() => {
    throw new Error('RPC forbidden')
  }),
}))
vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn(),
  executeTerraContractMulti: vi.fn(),
}))
vi.mock('@/utils/constants', async (original) => ({
  ...(await original<typeof import('@/utils/constants')>()),
  TREASURY_CONTRACT_ADDRESS: 'terra1treasury',
  WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1mapper',
  NATIVE_WRAPPED_PAIRS: { uluna: 'terra1wrapped' },
}))
const input = {
  fromToken: 'uluna',
  toToken: 'terra1wrapped',
  simRaw: '10000',
  maxSpreadStr: '0.05',
  slippageTolerancePercent: 5,
  maxMakerFills: 8,
  quoteTrader: { trader: 'terra1trader' },
}
function fixture(unwrap = false): IndexerRouteSolveResponse {
  return {
    token_in: unwrap ? 'terra1wrapped' : 'uluna',
    token_out: unwrap ? 'uluna' : 'terra1wrapped',
    hops: [],
    router_operations: [],
    intermediate_tokens: ['uluna', 'terra1wrapped'],
    estimated_amount_out: '9800',
    estimated_amount_out_net: '9800',
    spot_amount_out: '10000',
    slippage_percent: '2.0000',
    quote_kind: 'indexer_wrapper_db',
    tax_kind: 'none',
    buy_tax_bps: 0,
    sell_tax_bps: 0,
    router_hops_tax: false,
    execution_blocker: DIRECT_WRAPPER_FLOOR_UNSUPPORTED,
    execution_metadata: {
      version: 1,
      generation: 'wrapper-1',
      input_amount: '10000',
      max_spread: '0.05',
      protection_policy: 'quote_relative_v1',
      trader: 'terra1trader',
      observed_at_ms: Date.now() - 1000,
      expires_at_ms: Date.now() + 10000,
      policy: 'wrapper_only',
      execution_supported: false,
      hops: [],
      minimum_receive: '9310',
      wrapper: {
        input_token: unwrap ? 'terra1wrapped' : 'uluna',
        output_token: unwrap ? 'uluna' : 'terra1wrapped',
        input_amount: '10000',
        route_input_amount: unwrap ? '10000' : '9800',
        route_output_amount: unwrap ? '10000' : '9800',
        minimum_receive_base: unwrap ? '9400' : '9310',
        wrap_input: !unwrap,
        unwrap_output: unwrap,
        treasury: 'terra1treasury',
        mapper: 'terra1mapper',
      },
    },
  }
}
beforeEach(() => vi.clearAllMocks())
describe('zero-hop local wrapper arithmetic #1452', () => {
  it('rejects a revived old wrapper observation before pricing or signing (#1473)', () => {
    const idx = fixture()
    delete idx.execution_blocker
    const metadata = idx.execution_metadata!
    metadata.execution_supported = true
    metadata.wrapper!.floor_protocol_version = 1
    metadata.observed_at_ms = Date.now() - LOCAL_QUOTE_SIGNING_WINDOW_MS
    metadata.expires_at_ms = Date.now() + 10_000
    expect(() => validateWrapperOnlyQuote(idx, { ...input, trader: 'terra1trader' })).toThrow('Quote unavailable')
    expect(() =>
      assertLocalRouteSubmit(metadata, [], input.simRaw, input.maxSpreadStr, 'terra1trader', metadata.minimum_receive)
    ).toThrow('Quote unavailable')
  })
  it('pins quote-relative direct wrap ceil floors without a router market ceiling (#1473)', () => {
    const idx = fixture()
    idx.estimated_amount_out = '2'
    idx.estimated_amount_out_net = '2'
    idx.spot_amount_out = '2'
    delete idx.execution_blocker
    const metadata = idx.execution_metadata!
    metadata.input_amount = '2'
    metadata.protection_policy = 'quote_relative_v1'
    metadata.execution_supported = true
    metadata.minimum_receive = '2'
    Object.assign(metadata.wrapper!, {
      floor_protocol_version: 1,
      input_amount: '2',
      route_input_amount: '2',
      route_output_amount: '2',
      minimum_receive_base: '2',
    })
    expect(validateWrapperOnlyQuote(idx, { ...input, simRaw: '2', trader: 'terra1trader' })).toBe(metadata)
    assertLocalRouteSubmit(metadata, [], '2', '0.05', 'terra1trader', '2')
    metadata.minimum_receive = '1'
    metadata.wrapper!.minimum_receive_base = '1'
    expect(() => validateWrapperOnlyQuote(idx, { ...input, simRaw: '2', trader: 'terra1trader' })).toThrow(
      'Quote unavailable'
    )
    expect(() => assertLocalRouteSubmit(metadata, [], '2', '0.05', 'terra1trader', '1')).toThrow('Quote unavailable')
  })
  it.each([false, true])(
    'quotes and sizes with no RPC but never signs unprotected direct wrapper: unwrap=%s',
    async (unwrap) => {
      const idx = fixture(unwrap)
      const request = { ...input, fromToken: idx.token_in, toToken: idx.token_out }
      vi.mocked(getRouteSolve).mockResolvedValue(idx)
      const quote = await quoteCw20ViaRouteSolve(request)
      expect(quote).toMatchObject({
        return_amount: '9800',
        executionBlocker: DIRECT_WRAPPER_FLOOR_UNSUPPORTED,
        indexerOperations: [],
      })
      expect(evaluateTradingRisk(quote).availability).toBe('unavailable')
      idx.sizing_mode = 'local_pool_exact_output'
      idx.required_amount_in = '10000'
      idx.target_amount_out = '9800'
      vi.mocked(getRouteSolveExactOutput).mockResolvedValue(idx)
      expect(await quoteCw20ExactOutputViaRouteSolve({ ...request, targetRaw: '9800' })).toMatchObject({
        offerAmount: '10000',
        return_amount: '9800',
      })
      expect(() =>
        assertLocalRouteSubmit(quote!.executionMetadata, [], '10000', '0.05', 'terra1trader', '9400')
      ).toThrow('Quote unavailable')
      await expect(
        executeLocalNativeRoute('terra1trader', quote!.executionMetadata, [], '0.05', '9400')
      ).rejects.toThrow(DIRECT_WRAPPER_FLOOR_UNSUPPORTED)
      expect(queryContract).not.toHaveBeenCalled()
      expect(executeTerraContract).not.toHaveBeenCalled()
      expect(executeTerraContractMulti).not.toHaveBeenCalled()
    }
  )
  it('rejects forged executable metadata, stale generation context and mapping/floor changes', () => {
    for (const mutate of [
      (idx: IndexerRouteSolveResponse) => {
        delete idx.execution_metadata!.execution_supported
      },
      (idx: IndexerRouteSolveResponse) => {
        idx.execution_metadata!.expires_at_ms = Date.now() - 1
      },
      (idx: IndexerRouteSolveResponse) => {
        idx.execution_metadata!.wrapper!.route_output_amount = '9900'
      },
      (idx: IndexerRouteSolveResponse) => {
        idx.execution_metadata!.minimum_receive = '1'
      },
      (idx: IndexerRouteSolveResponse) => {
        idx.token_out = 'terra1other'
      },
      (idx: IndexerRouteSolveResponse) => {
        idx.spot_amount_out = '1000000'
        idx.slippage_percent = '0'
      },
    ]) {
      const idx = fixture()
      mutate(idx)
      expect(() => validateWrapperOnlyQuote(idx, { ...input, trader: 'terra1trader' })).toThrow('Quote unavailable')
    }
  })
})

describe('version 1 direct wrapper signing #1459', () => {
  function executable(unwrap = false) {
    const idx = fixture(unwrap)
    idx.execution_metadata!.execution_supported = true
    idx.execution_metadata!.wrapper!.floor_protocol_version = 1
    idx.execution_blocker = null // Exact supported backend wire shape.
    return idx
  }
  it.each([false, true])(
    'keeps original total risk and expiry on structural zero market movement: unwrap=%s',
    (unwrap) => {
      const q = executable(unwrap),
        sim = { return_amount: q.estimated_amount_out!, executionMetadata: q.execution_metadata!, riskEvidence: q }
      q.spot_amount_out = '11000'
      expect(evaluateTradingRisk(sim)).toMatchObject({ orderWarning: false, totalWarning: true, consentRequired: true })
      q.spot_amount_out = '15000'
      expect(evaluateTradingRisk(sim).expertRequired).toBe(true)
      expect(tradingRiskBlocksSubmit(evaluateTradingRisk(sim), false)).toBe(true)
      expect(tradingRiskBlocksSubmit(evaluateTradingRisk(sim), true)).toBe(false)
      q.execution_metadata!.expires_at_ms = Date.now() - 1
      expect(evaluateTradingRisk(sim).availability).toBe('unavailable')
    }
  )
  it.each([false, true])('serializes the exact reviewed wallet floor without RPC: unwrap=%s', async (unwrap) => {
    const idx = executable(unwrap)
    vi.mocked(getRouteSolve).mockResolvedValue(idx)
    const quote = await quoteCw20ViaRouteSolve({ ...input, fromToken: idx.token_in, toToken: idx.token_out })
    expect(quote!.executionBlocker).toBeUndefined()
    const risk = evaluateTradingRisk(quote)
    expect(risk).toMatchObject({ availability: 'available', orderPercent: 0, totalPercent: 2, consentRequired: false })
    expect(tradingRiskBlocksSubmit(risk, false)).toBe(false)
    expect(quote!.minimumReceive).toBe('9310')
    expect(() =>
      assertLocalRouteSubmit(quote!.executionMetadata, [], '10000', '0.05', 'terra1trader', '9310')
    ).not.toThrow()
    await executeLocalNativeRoute('terra1trader', quote!.executionMetadata!, [], '0.05', '9310')
    expect(executeTerraContract).toHaveBeenCalledTimes(1)
    if (unwrap) {
      const call = vi.mocked(executeTerraContract).mock.calls[0]
      expect(call[0]).toBe('terra1trader')
      expect(call[1]).toBe('terra1wrapped')
      const send = (call[2] as { send: { contract: string; amount: string; msg: string } }).send
      expect(send.contract).toBe('terra1mapper')
      expect(send.amount).toBe('10000')
      expect(atob(send.msg)).toBe('{"unwrap_with_minimum":{"recipient":null,"minimum_receive":"9310"}}')
      expect(call[3]).toBeUndefined()
      expect(call[4]).toMatchObject({ gasEstimation: 'offline' })
    } else {
      expect(executeTerraContract).toHaveBeenCalledWith(
        'terra1trader',
        'terra1treasury',
        { wrap_deposit_with_minimum: { minimum_receive: '9310' } },
        [{ denom: 'uluna', amount: '10000' }],
        expect.objectContaining({ gasEstimation: 'offline' })
      )
    }
    expect(queryContract).not.toHaveBeenCalled()
    expect(executeTerraContractMulti).not.toHaveBeenCalled()
  })
  it.each([false, true])(
    'rejects capability downgrade, altered floor, actor and stale plan before broadcast: unwrap=%s',
    async (unwrap) => {
      for (const mutate of [
        (idx: IndexerRouteSolveResponse) => {
          delete idx.execution_metadata!.wrapper!.floor_protocol_version
        },
        (idx: IndexerRouteSolveResponse) => {
          idx.execution_metadata!.execution_supported = false
        },
        (idx: IndexerRouteSolveResponse) => {
          idx.execution_metadata!.expires_at_ms = Date.now() - 1
        },
        (idx: IndexerRouteSolveResponse) => {
          idx.execution_metadata!.trader = 'terra1other'
        },
        (idx: IndexerRouteSolveResponse) => {
          idx.execution_metadata!.wrapper!.mapper = 'terra1other'
        },
        (idx: IndexerRouteSolveResponse) => {
          idx.execution_metadata!.wrapper!.input_amount = '9999'
        },
      ]) {
        const idx = executable(unwrap)
        mutate(idx)
        await expect(
          executeLocalNativeRoute('terra1trader', idx.execution_metadata!, [], '0.05', '9310')
        ).rejects.toThrow()
      }
      for (const floor of ['0', '9309', '9400']) {
        const idx = executable(unwrap)
        await expect(
          executeLocalNativeRoute('terra1trader', idx.execution_metadata!, [], '0.05', floor)
        ).rejects.toThrow('Quote unavailable')
      }
      expect(executeTerraContract).not.toHaveBeenCalled()
      expect(executeTerraContractMulti).not.toHaveBeenCalled()
      expect(queryContract).not.toHaveBeenCalled()
    }
  )
  it('rejects a capability claim combined with a legacy blocker', () => {
    const idx = executable()
    idx.execution_blocker = DIRECT_WRAPPER_FLOOR_UNSUPPORTED
    expect(() => validateWrapperOnlyQuote(idx, { ...input, trader: 'terra1trader' })).toThrow('Quote unavailable')
  })
})
