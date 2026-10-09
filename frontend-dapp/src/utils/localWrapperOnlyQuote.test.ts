import { beforeEach, describe, expect, it, vi } from 'vitest'
import { getRouteSolve, getRouteSolveExactOutput } from '@/services/indexer/client'
import { queryContract } from '@/services/terraclassic/queries'
import { executeTerraContract, executeTerraContractMulti } from '@/services/terraclassic/transactions'
import { executeLocalNativeRoute } from '@/services/terraclassic/router'
import type { IndexerRouteSolveResponse } from '@/types'
import { quoteCw20ExactOutputViaRouteSolve, quoteCw20ViaRouteSolve } from './cw20RouteSolveQuote'
import { assertLocalRouteSubmit } from './localRouteQuote'
import { DIRECT_WRAPPER_FLOOR_UNSUPPORTED, validateWrapperOnlyQuote } from './localWrapperOnlyQuote'
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
