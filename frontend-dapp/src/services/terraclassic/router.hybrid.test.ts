import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>()
  return {
    ...actual,
    ROUTER_CONTRACT_ADDRESS: 'terra1router_mock',
  }
})

vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn(),
}))

import { executeMultiHopSwap } from './router'
import { executeTerraContract } from './transactions'
import type { SwapOperation } from './router'

const mockedExecute = vi.mocked(executeTerraContract)

const WALLET = 'terra1walletaddr'
const TOKEN_IN = 'terra1tokena'
const terraA = { token: { contract_addr: 'terra1aaa' } }
const terraB = { token: { contract_addr: 'terra1bbb' } }

describe('executeMultiHopSwap hybrid message shape (GitLab #84 / #199)', () => {
  beforeEach(() => {
    mockedExecute.mockReset()
    mockedExecute.mockResolvedValueOnce('txhash_router_hybrid')
  })

  it('serializes hybrid on terra_swap and omits null book_start_hint', async () => {
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: terraA,
          ask_asset_info: terraB,
          hybrid: {
            pool_input: '600',
            book_input: '400',
            max_maker_fills: 8,
            book_start_hint: null,
          },
        },
      },
    ]

    await executeMultiHopSwap(WALLET, TOKEN_IN, '1000', ops, '0.05', '900', 'terra1recv', 1_700_000_000)

    expect(mockedExecute).toHaveBeenCalledTimes(1)
    const msg = mockedExecute.mock.calls[0][2] as { send: { msg: string } }
    const decoded = JSON.parse(atob(msg.send.msg)) as {
      execute_swap_operations: {
        operations: Array<{ terra_swap: Record<string, unknown> }>
        max_spread: string
        minimum_receive?: string
        to?: string
        deadline?: number
      }
    }

    expect(decoded).toEqual({
      execute_swap_operations: {
        operations: [
          {
            terra_swap: {
              offer_asset_info: terraA,
              ask_asset_info: terraB,
              hybrid: {
                pool_input: '600',
                book_input: '400',
                max_maker_fills: 8,
              },
            },
          },
        ],
        max_spread: '0.05',
        minimum_receive: '900',
        to: 'terra1recv',
        deadline: 1_700_000_000,
      },
    })
  })

  it('includes book_start_hint when set on a hop', async () => {
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: terraA,
          ask_asset_info: terraB,
          hybrid: {
            pool_input: '500',
            book_input: '500',
            max_maker_fills: 4,
            book_start_hint: 99,
          },
        },
      },
    ]

    await executeMultiHopSwap(WALLET, TOKEN_IN, '1000', ops, '0.01')

    const msg = mockedExecute.mock.calls[0][2] as { send: { msg: string } }
    const hybrid = JSON.parse(atob(msg.send.msg)).execute_swap_operations.operations[0].terra_swap.hybrid
    expect(hybrid).toEqual({
      pool_input: '500',
      book_input: '500',
      max_maker_fills: 4,
      book_start_hint: 99,
    })
  })

  it('omits hybrid key on pool-only hops in multi-hop router msg', async () => {
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: terraA,
          ask_asset_info: terraB,
          hybrid: {
            pool_input: '1000',
            book_input: '0',
            max_maker_fills: 8,
            book_start_hint: null,
          },
        },
      },
      {
        terra_swap: {
          offer_asset_info: terraB,
          ask_asset_info: { token: { contract_addr: 'terra1ccc' } },
        },
      },
    ]

    await executeMultiHopSwap(WALLET, TOKEN_IN, '1000', ops, '0.05')

    const msg = mockedExecute.mock.calls[0][2] as { send: { msg: string } }
    const operations = JSON.parse(atob(msg.send.msg)).execute_swap_operations.operations
    expect(operations[0].terra_swap.hybrid).toBeDefined()
    expect(operations[1].terra_swap.hybrid).toBeUndefined()
  })
})
