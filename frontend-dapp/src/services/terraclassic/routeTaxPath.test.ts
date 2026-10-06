import { beforeEach, describe, expect, it, vi } from 'vitest'
import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import { getPair } from './factory'
import { assertRouteTaxFunding, previewRouteTax, ROUTE_TAX_FUNDING_UNAVAILABLE } from './routeTaxFunding'
import { assertRouteTaxPath, REPEATED_ROUTE_TAX_ASSET } from './routeTaxPath'
import { executeMultiHopSwap, reverseSimulateMultiHopSwap, simulateMultiHopSwap, type SwapOperation } from './router'
import { enrichSwapOperationsWithHopMinReturns, preflightSwapRouteSpread } from './swapRoutePreflight'

vi.mock('./queries', () => ({ queryContract: vi.fn() }))
vi.mock('./transactions', () => ({ executeTerraContract: vi.fn(), executeTerraContractMulti: vi.fn() }))
vi.mock('./factory', () => ({ getPair: vi.fn() }))
vi.mock('./pair', () => ({ simulateHybridSwap: vi.fn() }))

const taxed = 'terra1taxed'
const ordinary = 'terra1ordinary'
const other = 'terra1other'
const third = 'terra1third'
function route(path: string[]): SwapOperation[] {
  return path.slice(1).map((ask, index) => ({
    terra_swap: {
      offer_asset_info: { token: { contract_addr: path[index] } },
      ask_asset_info: { token: { contract_addr: ask } },
    },
  }))
}

describe('logical route-tax asset uniqueness (#1390)', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(queryContract).mockImplementation(async (token, msg) => {
      if ('config' in msg) return { funding_version: 1 } as never
      if ('route_tax_capability' in msg && token === taxed) {
        return { funding_version: 1, router: 'terra1router', buy_bps: 0, sell_bps: 0 } as never
      }
      if ('simulate_swap_operations' in msg || 'reverse_simulate_swap_operations' in msg) {
        return { amount: '1000' } as never
      }
      throw new Error('unknown variant')
    })
  })

  it.each([
    [taxed, ordinary, other],
    [ordinary, taxed, other],
    [ordinary, other, taxed],
    [ordinary, other, ordinary, taxed],
  ])('supports one taxed position and ordinary cycles: %j', async (...path) => {
    expect(await assertRouteTaxFunding(route(path))).toBe(1)
    expect(await simulateMultiHopSwap('1000', route(path))).toMatchObject({ amount: '1000', taxFundingVersion: 1 })
    expect(await reverseSimulateMultiHopSwap('1000', route(path))).toMatchObject({
      amount: '1000',
      taxFundingVersion: 1,
    })
  })

  it.each([
    [taxed, ordinary, taxed],
    [taxed, ordinary, taxed, other],
    [ordinary, taxed, other, taxed],
    [ordinary, taxed, other, taxed, third],
  ])('rejects repeated zero-rate capable tokens: %j', async (...path) => {
    await expect(assertRouteTaxFunding(route(path))).rejects.toThrow(REPEATED_ROUTE_TAX_ASSET)
  })

  it('blocks execution, forward/reverse quotes and client pair quotes before doing economic work', async () => {
    const operations = route([taxed, ordinary, taxed])
    const trader = { trader: 'terra1synthetic', taxFundingVersion: 1 as const }
    await expect(executeMultiHopSwap('terra1synthetic', taxed, '1000', operations, '0.01')).rejects.toThrow(
      REPEATED_ROUTE_TAX_ASSET
    )
    await expect(simulateMultiHopSwap('1000', operations)).rejects.toThrow(REPEATED_ROUTE_TAX_ASSET)
    await expect(reverseSimulateMultiHopSwap('1000', operations)).rejects.toThrow(REPEATED_ROUTE_TAX_ASSET)
    await expect(preflightSwapRouteSpread(operations, '1000', '0.01', trader)).rejects.toThrow(REPEATED_ROUTE_TAX_ASSET)
    await expect(enrichSwapOperationsWithHopMinReturns(operations, '1000', 1, trader)).rejects.toThrow(
      REPEATED_ROUTE_TAX_ASSET
    )
    expect(executeTerraContract).not.toHaveBeenCalled()
    expect(getPair).not.toHaveBeenCalled()
    expect(vi.mocked(queryContract).mock.calls.some(([, msg]) => 'simulate_swap_operations' in msg)).toBe(false)
    expect(vi.mocked(queryContract).mock.calls.some(([, msg]) => 'reverse_simulate_swap_operations' in msg)).toBe(false)
  })

  it('does not infer ordinary identity from legacy zero tax rates', async () => {
    vi.mocked(queryContract).mockImplementation(async (_token, msg) => {
      if ('get_config' in msg) return { buy_bps: 0, sell_bps: 0, transfer_bps: 0 } as never
      throw new Error('unknown variant')
    })
    await expect(assertRouteTaxPath(route([taxed, ordinary, taxed]))).rejects.toThrow(REPEATED_ROUTE_TAX_ASSET)
  })

  it('blocks repeated capable tokens before using the legacy zero-rate fallback', async () => {
    vi.mocked(queryContract).mockImplementation(async (_token, msg) => {
      if ('config' in msg) return {} as never
      if ('route_tax_capability' in msg) return { funding_version: 1, buy_bps: 0, sell_bps: 0 } as never
      if ('get_config' in msg) return { buy_bps: 0, sell_bps: 0, transfer_bps: 0 } as never
      throw new Error('unknown variant')
    })
    await expect(assertRouteTaxFunding(route([taxed, ordinary, taxed]))).rejects.toThrow(REPEATED_ROUTE_TAX_ASSET)
  })

  it.each(['timeout', '503 unavailable'])('fails closed on repeated asset discovery: %s', async (message) => {
    vi.mocked(queryContract).mockRejectedValue(new Error(message))
    await expect(assertRouteTaxPath(route([taxed, ordinary, taxed]))).rejects.toThrow(message)
  })

  it.each(['sell', 'buy'] as const)('propagates protocol-exempt trader preview rejection for %s', async (kind) => {
    vi.mocked(queryContract).mockRejectedValue(new Error('protocol-exempt trader cannot fund a route'))
    await expect(
      previewRouteTax({ token: taxed, kind, amount: '1000', trader: 'terra1exempt', pair: 'terra1pair' })
    ).rejects.toThrow(ROUTE_TAX_FUNDING_UNAVAILABLE)
  })
})
