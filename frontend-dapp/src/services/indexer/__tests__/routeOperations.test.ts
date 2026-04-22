import { describe, it, expect } from 'vitest'
import { swapOperationsFromIndexerResponse } from '../routeOperations'

describe('swapOperationsFromIndexerResponse', () => {
  it('maps indexer router_operations to SwapOperation[]', () => {
    const terraA = 'terra1aaa'
    const terraB = 'terra1bbb'
    const ops = swapOperationsFromIndexerResponse(
      [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: terraA } },
            ask_asset_info: { token: { contract_addr: terraB } },
            hybrid: null,
          },
        },
      ],
      1
    )
    expect(ops).toHaveLength(1)
    expect(ops[0].terra_swap.offer_asset_info).toEqual({ token: { contract_addr: terraA } })
    expect(ops[0].terra_swap.ask_asset_info).toEqual({ token: { contract_addr: terraB } })
    expect(ops[0].terra_swap.hybrid).toBeUndefined()
  })

  it('parses hybrid when present', () => {
    const ops = swapOperationsFromIndexerResponse(
      [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: 'terra1a' } },
            ask_asset_info: { token: { contract_addr: 'terra1b' } },
            hybrid: {
              pool_input: '7',
              book_input: '3',
              max_maker_fills: 8,
              book_start_hint: null,
            },
          },
        },
      ],
      1
    )
    expect(ops[0].terra_swap.hybrid).toEqual({
      pool_input: '7',
      book_input: '3',
      max_maker_fills: 8,
      book_start_hint: null,
    })
  })

  it('throws on hop count mismatch', () => {
    expect(() =>
      swapOperationsFromIndexerResponse(
        [
          {
            terra_swap: {
              offer_asset_info: { token: { contract_addr: 'a' } },
              ask_asset_info: { token: { contract_addr: 'b' } },
            },
          },
        ],
        2
      )
    ).toThrow('one entry per hop')
  })
})
