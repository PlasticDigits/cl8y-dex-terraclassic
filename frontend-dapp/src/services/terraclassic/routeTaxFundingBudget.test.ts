import { quotePayInvoice } from '@/utils/payInvoice'
import { assertRouteTaxFunding } from './routeTaxFunding'
import { getGasLimitForTx, gasLimitForRouteLocalSwapOperations } from './terraGas'
import { messageUsesSwapAutoGas } from './swapAutoGas'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { simulateHybridSwap } from './pair'
import { executeMultiHopSwap, simulateMultiHopSwap, reverseSimulateMultiHopSwap, type SwapOperation } from './router'
import { enrichSwapOperationsWithHopMinReturns, preflightSwapRouteSpread } from './swapRoutePreflight'

vi.mock('./queries', () => ({ queryContract: vi.fn() }))
vi.mock('./transactions', () => ({ executeTerraContract: vi.fn(), executeTerraContractMulti: vi.fn() }))
vi.mock('./factory', () => ({ getPair: vi.fn(async () => ({ contract_addr: 'terra1pair' })) }))
vi.mock('./pair', () => ({ simulateHybridSwap: vi.fn() }))
const path = ['terra1cl8y', 'terra1alpha', 'terra1ust1', 'terra1ustr']
const operations: SwapOperation[] = path.slice(1).map((token, i) => ({
  terra_swap: {
    offer_asset_info: { token: { contract_addr: path[i] } },
    ask_asset_info: { token: { contract_addr: token } },
  },
}))

describe('gross/economic/net frontend parity (#1381)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(queryContract).mockImplementation(async (token, msg) => {
      if ('config' in msg) return { funding_version: 1 } as never
      if ('reverse_simulate_swap_operations' in msg) return { amount: '1010000' } as never
      if ('simulate_swap_operations' in msg) {
        const input = msg.simulate_swap_operations as { offer_amount: string; operations: SwapOperation[] }
        let economic = BigInt(input.offer_amount)
        while (economic + economic / 100n > BigInt(input.offer_amount)) economic--
        const hop = input.operations[0].terra_swap
        if (hop.hybrid && BigInt(hop.hybrid.pool_input) + BigInt(hop.hybrid.book_input) !== economic) {
          throw new Error('Hybrid swap split mismatch')
        }
        if (hop.hybrid && BigInt(hop.hybrid.book_input) > 0n && !hop.min_return) {
          throw new Error('Book leg missing slippage floor')
        }
        return { amount: (economic * 2n).toString() } as never
      }
      const p = msg.route_tax_preview as { kind: string; amount: string }
      if (token !== 'terra1alpha') throw new Error('unknown variant route_tax_preview')
      const gross = BigInt(p.amount)
      let economic = gross
      let net = gross
      if (p.kind === 'sell') {
        while (economic + economic / 100n > gross) economic--
        net = economic
      } else net = gross - (gross * 450n) / 10000n
      return {
        funding_version: 1,
        gross: p.amount,
        economic: economic.toString(),
        net: net.toString(),
        tax: (p.kind === 'sell' ? economic / 100n : gross - net).toString(),
      } as never
    })
    vi.mocked(simulateHybridSwap).mockImplementation(
      async (_pair, _asset, offer) =>
        ({
          return_amount: (BigInt(offer) * 2n).toString(),
          pool_return_amount: (BigInt(offer) * 2n).toString(),
          book_return_amount: '0',
          pool_spread_amount: '0',
          spread_amount: '0',
          commission_amount: '0',
          pool_commission_amount: '0',
          book_commission_amount: '0',
        }) as never
    )
    vi.mocked(executeTerraContract).mockResolvedValue('synthetic-tx')
  })
  it('preflight and enrichment spend net acquired ALPHA, then its funded economic sell', async () => {
    const trader = { trader: 'terra1synthetic', taxFundingVersion: 1 as const }
    await enrichSwapOperationsWithHopMinReturns(operations, '1000', 1, trader)
    expect(vi.mocked(simulateHybridSwap).mock.calls.map((args) => args[2])).toEqual(['1000', '1892', '3784'])
    vi.mocked(simulateHybridSwap).mockClear()
    const preflight = await preflightSwapRouteSpread(operations, '1000', '0.01', trader)
    expect(preflight.anyHopExceedsMaxSpread).toBe(false)
    expect(vi.mocked(simulateHybridSwap).mock.calls.map((args) => args[2])).toEqual(['1000', '1892', '3784'])
  })
  it('book minimum return is post-buy-tax net and gross hybrid is resized for the sell', async () => {
    const trader = { trader: 'terra1synthetic', taxFundingVersion: 1 as const }
    const bookRoute = operations.map((op, i) => ({
      terra_swap: {
        ...op.terra_swap,
        ...(i === 0 ? { hybrid: { pool_input: '500', book_input: '500', max_maker_fills: 8 } } : {}),
      },
    }))
    const enriched = await enrichSwapOperationsWithHopMinReturns(bookRoute, '1000', 1, trader)
    expect(enriched[0].terra_swap.min_return).toBe('1890')
    const taxedFirst = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: path[1] } },
          ask_asset_info: { token: { contract_addr: path[2] } },
          hybrid: { pool_input: '510000', book_input: '500000', max_maker_fills: 8 },
        },
      },
    ]
    vi.mocked(simulateHybridSwap).mockClear()
    await enrichSwapOperationsWithHopMinReturns(taxedFirst, '1010000', 1, trader)
    expect(vi.mocked(simulateHybridSwap).mock.calls[0][2]).toBe('1000000')
    expect(vi.mocked(simulateHybridSwap).mock.calls[0][3]).toMatchObject({ pool_input: '504951', book_input: '495049' })
  })

  it('real router builder authorizes exactly gross input with a versioned hook', async () => {
    await executeMultiHopSwap('terra1synthetic', path[0], '1010000', operations, '0.01', '999')
    const msg = vi.mocked(executeTerraContract).mock.calls[0][2] as { send: { amount: string; msg: string } }
    expect(msg.send.amount).toBe('1010000')
    expect(getGasLimitForTx(msg as unknown as Record<string, unknown>)).toBe(gasLimitForRouteLocalSwapOperations(3))
    expect(messageUsesSwapAutoGas(msg as unknown as Record<string, unknown>)).toBe(true)
    expect(JSON.parse(atob(msg.send.msg))).toMatchObject({
      execute_route_swap_operations: {
        funding_version: 1,
        minimum_receive: '999',
      },
    })
  })
  it('replans a taxed invoice frozen book ratio at final slippage gross and verifies that exact plan', async () => {
    const invoice = {
      invoiceToken: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
      invoiceAmount: '1000000',
      payee: 'terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l',
      hookMsg: '',
    }
    const seed: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: path[1] } },
          ask_asset_info: { token: { contract_addr: invoice.invoiceToken } },
          hybrid: { pool_input: '800000', book_input: '200000', max_maker_fills: 8 },
        },
      },
    ]
    const quote = await quotePayInvoice(
      { invoice, payToken: path[1], slippagePercent: 1, trader: 'terra1synthetic', payTokenBalance: '1020100' },
      {
        getRouteSolve: vi.fn(async () => ({
          token_in: path[1],
          token_out: invoice.invoiceToken,
          hops: [{ pair: 'terra1pair', offer_token: path[1], ask_token: invoice.invoiceToken }],
          router_operations: seed,
        })),
        reverseSimulate: reverseSimulateMultiHopSwap,
        forwardSimulate: simulateMultiHopSwap,
        assertFunding: assertRouteTaxFunding,
        enrichOperations: enrichSwapOperationsWithHopMinReturns,
      }
    )
    expect(quote).toMatchObject({
      status: 'ok',
      payRaw: '1020100',
      cw20SendAmount: '1020100',
      taxFundingVersion: 1,
      operations: [
        {
          terra_swap: {
            hybrid: { pool_input: '808000', book_input: '202000', max_maker_fills: 8 },
            min_return: '1999800',
          },
        },
      ],
    })
    if (quote.status !== 'ok') throw new Error('Funded invoice quote was unavailable')
    // Exercise the real versioned Send builder with the newly sized plan.
    const previous = vi.mocked(executeTerraContract).mock.calls.length
    await executeMultiHopSwap(
      'terra1synthetic',
      path[1],
      quote.cw20SendAmount,
      quote.operations,
      '0.01',
      invoice.invoiceAmount
    )
    const msg = vi.mocked(executeTerraContract).mock.calls[previous][2] as { send: { amount: string; msg: string } }
    expect(msg.send.amount).toBe(quote.payRaw)
    expect(JSON.parse(atob(msg.send.msg))).toMatchObject({
      execute_route_swap_operations: {
        funding_version: 1,
        operations: [
          {
            terra_swap: {
              hybrid: {
                pool_input: '808000',
                book_input: '202000',
                max_maker_fills: 8,
              },
              min_return: '1999800',
            },
          },
        ],
        minimum_receive: invoice.invoiceAmount,
      },
    })
  })

  it('rejects legacy taxed fallback without signing or automatic resend', async () => {
    vi.mocked(queryContract).mockResolvedValueOnce({}).mockResolvedValueOnce({ sell_bps: 100 })
    await expect(executeMultiHopSwap('terra1synthetic', path[0], '1010000', operations, '0.01')).rejects.toThrow(
      'Route tax funding could not be verified'
    )
    expect(executeTerraContract).not.toHaveBeenCalled()
  })
})
