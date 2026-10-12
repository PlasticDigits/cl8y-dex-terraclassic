import { afterEach, describe, expect, it, vi } from 'vitest'
import { offlineSwapGasLimit, OFFLINE_SWAP_BUDGET_UNAVAILABLE } from '../offlineSwapBudget'
import { buildTerraClassicFee, estimateFeeUlunaAmountForGasLimit } from '../terraGas'
import { executeMultiHopSwap } from '../router'
import { estimateSwapNetworkFee } from '../swapNetworkFee'
import { installSwapGasReaderForTests } from '../swapAutoGas'
import { localQuote, LOCAL_FROM, LOCAL_TO, LOCAL_WALLET } from '@/test/localRouteFixtures'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { evaluateSwapNativeGasGate } from '@/utils/swapNativeGasBalanceGate'

const { wallet } = vi.hoisted(() => ({
  wallet: {
    address: 'terra1wallet000000000000000000000000001',
    getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 1n }),
    broadcastTx: vi.fn().mockResolvedValue('INCLUDED'),
    pollTx: vi.fn().mockResolvedValue({ txResponse: { code: 0 } }),
  },
}))
vi.mock('../wallet', () => ({ getConnectedWallet: () => wallet }))

function route(payload: Record<string, unknown>) {
  return [{ msg: { send: { msg: btoa(JSON.stringify({ execute_swap_operations: payload })) } } }]
}
afterEach(() => {
  vi.clearAllMocks()
  installSwapGasReaderForTests(null)
})

describe('offline execution admission (#1480)', () => {
  it.each([
    {},
    { unknown_protocol: {} },
    { swap: {}, increase_allowance: {} },
    { send: { msg: 'not base64' } },
    { send: { msg: btoa('null') } },
    { send: { msg: 'A'.repeat(65_537) } },
    { execute_route_swap_operations: { funding_version: 2, operations: [{ terra_swap: {} }] } },
    { execute_swap_operations: { funding_version: 1, operations: [{ terra_swap: {} }] } },
  ])('rejects an unknown/malformed family before signing %j', (msg) => {
    expect(() => offlineSwapGasLimit([{ msg }])).toThrow(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  })
  it.each([0, 5, 100])('rejects unsupported hop count %s', (hops) => {
    expect(() =>
      offlineSwapGasLimit(route({ operations: Array.from({ length: hops }, () => ({ terra_swap: {} })) }))
    ).toThrow(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  })
  it.each([0, -1, 1.5, NaN, Infinity, 101, '8'])('rejects invalid maker work %s', (makers) => {
    expect(() =>
      offlineSwapGasLimit([
        {
          msg: {
            swap: {
              hybrid: {
                pool_input: '1',
                book_input: '1',
                max_maker_fills: makers,
              },
            },
          },
        },
      ])
    ).toThrow(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  })
  it('rejects over-limit work rather than clamping the signed transaction', () => {
    expect(() =>
      offlineSwapGasLimit(
        route({
          operations: Array.from({ length: 4 }, () => ({
            terra_swap: {
              hybrid: { pool_input: '1', book_input: '1', max_maker_fills: 100 },
            },
          })),
        })
      )
    ).toThrow(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  })
  it.each([
    { pool_input: ((1n << 128n) - 1n).toString(), book_input: '1', max_maker_fills: 1 },
    { pool_input: (1n << 128n).toString(), book_input: '0', max_maker_fills: 1 },
    { pool_input: '1', book_input: '1', max_maker_fills: 1, arbitrary_work_cap: 1_000_000 },
  ])('rejects overflow or unknown work-bearing fields %j', (hybrid) => {
    expect(() => offlineSwapGasLimit([{ msg: { swap: { hybrid } } }])).toThrow(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  })
  it('rejects four-hop greedy and native book work within otherwise valid shapes', () => {
    const operations = Array.from({ length: 4 }, () => ({ terra_swap: { greedy: { max_maker_fills: 1 } } }))
    expect(() => offlineSwapGasLimit(route({ operations }))).toThrow(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
    expect(() =>
      offlineSwapGasLimit([{ msg: { wrap_deposit: {} } }, ...route({ operations: operations.slice(0, 1) })])
    ).toThrow(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  })
  it.each([0, -1, 0.5, NaN, Infinity, Number.MAX_SAFE_INTEGER])('checked fee arithmetic rejects %s', (gas) => {
    expect(() => buildTerraClassicFee(gas)).toThrow()
  })
})

describe('actual reviewed router builder → broadcast (#1480)', () => {
  it.each([undefined, 1] as const)(
    'one-hop protocol %s fee row/reserve/requested Fee agrees without simulation',
    async (version) => {
      const reader = vi.fn(() => {
        throw new Error('Request simulation forbidden')
      })
      installSwapGasReaderForTests(reader)
      const quote = localQuote(LOCAL_FROM, LOCAL_TO, '1000000', { trader: LOCAL_WALLET })
      quote.execution_metadata!.tax_funding_version = version
      const ops = quote.router_operations!
      const estimate = estimateSwapNetworkFee({
        isDirectWrap: false,
        needsWrapInput: false,
        cw20RouterOperations: ops,
        localExecution: true,
        taxFundingVersion: version,
      })
      const expectedGas = version === 1 ? 2_600_000n : 3_200_000n
      // Independent exact-price calculation, including fee denom and integer rounding.
      const expectedFee = (expectedGas * 28_325n + 999n) / 1000n
      expect(estimate.feeUluna).toBe(expectedFee)
      await executeMultiHopSwap(
        LOCAL_WALLET,
        LOCAL_FROM,
        '1000000',
        ops,
        '0.05',
        '950000',
        undefined,
        Math.floor(Date.now() / 1000) + 30,
        quote.execution_metadata
      )
      expect(reader).not.toHaveBeenCalled()
      expect(wallet.broadcastTx).toHaveBeenCalledTimes(1)
      const fee = wallet.broadcastTx.mock.calls[0][1] as {
        gasLimit: bigint
        amount: Array<{ denom: string; amount: string }>
      }
      expect(fee.gasLimit).toBe(expectedGas)
      expect(fee.amount).toEqual([{ denom: 'uluna', amount: expectedFee.toString() }])
      const unsigned = wallet.broadcastTx.mock.calls[0][0] as {
        msgs: Array<{ data: { msg: { send: { msg: string; amount: string } } } }>
      }
      const send = unsigned.msgs[0].data.msg.send
      const payload = JSON.parse(atob(send.msg))[
        version === 1 ? 'execute_route_swap_operations' : 'execute_swap_operations'
      ]
      expect(send.amount).toBe('1000000')
      expect(payload.minimum_receive).toBe('950000')
      expect(payload.operations[0].terra_swap.min_return).toBe('950000')
      expect(payload.max_spread).toBe(quote.execution_metadata!.execution_market_spread)
      expect(payload.deadline).toBe(Math.floor(quote.execution_metadata!.expires_at_ms / 1000))
      expect(quote.execution_metadata!.generation).toBe('fixture1')
      const balance = { data: expectedFee.toString(), isLoading: false, isError: false }
      expect(evaluateSwapNativeGasGate('1', 6, false, '1000000', balance, expectedFee).canSubmit).toBe(true)
      expect(
        evaluateSwapNativeGasGate(
          '1',
          6,
          false,
          '1000000',
          { ...balance, data: (expectedFee - 1n).toString() },
          expectedFee
        ).canSubmit
      ).toBe(false)
    }
  )
  it('native Max and pay+fee ±1 use the reviewed fee without stale hop/funding hints', () => {
    const fee = estimateFeeUlunaAmountForGasLimit(800_000)
    const max = computeMaxSpendableHumanAmount({
      balanceRaw: (fee + 1_000_000n).toString(),
      decimals: 6,
      assetIsNativeUluna: true,
      context: 'swap_native',
      executionFeeUluna: fee,
      nativeSwapHints: { isDirectWrap: true, needsWrapInput: false },
    })
    expect(max.human).toBe('1')
    expect(max.reserveUluna).toBe(22_660_000n)
    for (const delta of [-1n, 0n, 1n]) {
      const balance = { data: (fee + 1_000_000n + delta).toString(), isLoading: false, isError: false }
      expect(evaluateSwapNativeGasGate('1', 6, true, '1000000', balance, fee).canSubmit).toBe(delta >= 0n)
    }
  })
})
