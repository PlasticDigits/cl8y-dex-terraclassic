import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/factory', () => ({
  getPair: vi.fn(),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  simulateHybridSwap: vi.fn(),
}))

import { getPair } from '@/services/terraclassic/factory'
import { simulateHybridSwap } from '@/services/terraclassic/pair'
import { preflightSwapRouteSpread } from '@/services/terraclassic/swapRoutePreflight'
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
})
