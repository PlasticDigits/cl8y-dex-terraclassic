import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/factory', () => ({
  getPair: vi.fn(),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  simulateHybridSwap: vi.fn(),
}))

import { getPair } from '@/services/terraclassic/factory'
import { simulateHybridSwap } from '@/services/terraclassic/pair'
import { preflightSwapRouteSpread, computeDirectHybridMinReturn } from '@/services/terraclassic/swapRoutePreflight'
import type { SwapOperation } from '@/services/terraclassic/router'

const mockedGetPair = vi.mocked(getPair)
const mockedSim = vi.mocked(simulateHybridSwap)

const WALLET = 'terra1wallet000000000000000000000000000000'
const PAIR = 'terra1pair000000000000000000000000000000'
const TOKEN_A = 'terra1tokena000000000000000000000000000'
const TOKEN_B = 'terra1tokenb000000000000000000000000000'

beforeEach(() => {
  vi.clearAllMocks()
})

describe('preflightSwapRouteSpread', () => {
  it('forwards trader on each hop hybrid_simulation (GitLab #245)', async () => {
    mockedGetPair.mockResolvedValueOnce({ contract_addr: PAIR } as never)
    mockedSim.mockResolvedValueOnce({
      return_amount: '900',
      spread_amount: '10',
      commission_amount: '5',
      book_return_amount: '0',
      pool_return_amount: '900',
    })

    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: TOKEN_A } },
          ask_asset_info: { token: { contract_addr: TOKEN_B } },
        },
      },
    ]

    await preflightSwapRouteSpread(ops, '1000', '0.01', { trader: WALLET })

    expect(mockedSim).toHaveBeenCalledWith(
      PAIR,
      ops[0].terra_swap.offer_asset_info,
      '1000',
      expect.objectContaining({ pool_input: '1000', book_input: '0' }),
      { trader: WALLET }
    )
  })

  it('does not reject pool-only multihop before simulation (#341)', async () => {
    mockedGetPair.mockResolvedValue({ contract_addr: PAIR } as never)
    mockedSim.mockResolvedValue({
      return_amount: '900',
      spread_amount: '2',
      commission_amount: '5',
      book_return_amount: '0',
      pool_return_amount: '900',
    })

    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: TOKEN_A } },
          ask_asset_info: { token: { contract_addr: TOKEN_B } },
          hybrid: { pool_input: '1000', book_input: '0', max_maker_fills: 1, book_start_hint: null },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: TOKEN_B } },
          ask_asset_info: { token: { contract_addr: TOKEN_A } },
        },
      },
    ]

    const result = await preflightSwapRouteSpread(ops, '1000', '0.005', { trader: WALLET })
    expect(result.anyHopExceedsMaxSpread).toBe(false)
  })
})

describe('direct post-hook net floor (#1398)', () => {
  it('signs the post-hook quote for pool-only swaps', async () => {
    mockedSim.mockResolvedValueOnce({ return_amount: '900', pool_return_amount: '1000' } as never)
    expect(await computeDirectHybridMinReturn(PAIR, { token: { contract_addr: TOKEN_A } }, '1000', undefined, 1)).toBe(
      '891'
    )
    expect(mockedSim).toHaveBeenCalledWith(
      PAIR,
      { token: { contract_addr: TOKEN_A } },
      '1000',
      expect.objectContaining({ pool_input: '1000', book_input: '0' }),
      undefined
    )
  })

  it('uses total post-hook output for a mixed split', async () => {
    mockedSim.mockResolvedValueOnce({
      return_amount: '900',
      pool_return_amount: '500',
      book_return_amount: '500',
    } as never)
    const hybrid = { pool_input: '500', book_input: '500', max_maker_fills: 8, book_start_hint: null }
    expect(await computeDirectHybridMinReturn(PAIR, { token: { contract_addr: TOKEN_A } }, '1000', hybrid, 1)).toBe(
      '891'
    )
  })

  it('blocks signing when deductions or rounding leave no positive output', async () => {
    for (const amount of ['0', '1']) {
      mockedSim.mockResolvedValueOnce({ return_amount: amount } as never)
      await expect(
        computeDirectHybridMinReturn(PAIR, { token: { contract_addr: TOKEN_A } }, '1000', undefined, 1)
      ).rejects.toThrow('no receivable output')
    }
  })
})
