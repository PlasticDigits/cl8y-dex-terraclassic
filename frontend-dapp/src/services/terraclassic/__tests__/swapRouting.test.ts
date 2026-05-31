import { describe, expect, it } from 'vitest'
import { hybridFromSingleHopIndexerOps, swapOpsRequireRouter } from '../swapRouting'
import type { SwapOperation } from '../router'

const singleHop: SwapOperation[] = [
  {
    terra_swap: {
      offer_asset_info: { token: { contract_addr: 'terra1a' } },
      ask_asset_info: { token: { contract_addr: 'terra1b' } },
      hybrid: { pool_input: '1', book_input: '2', max_maker_fills: 4, book_start_hint: null },
    },
  },
]

describe('swapRouting (GitLab #249)', () => {
  it('requires router only for 2+ hops', () => {
    expect(swapOpsRequireRouter(undefined)).toBe(false)
    expect(swapOpsRequireRouter(singleHop)).toBe(false)
    expect(swapOpsRequireRouter([...singleHop, ...singleHop])).toBe(true)
  })

  it('extracts hybrid from single-hop indexer ops', () => {
    expect(hybridFromSingleHopIndexerOps(singleHop)?.max_maker_fills).toBe(4)
    expect(hybridFromSingleHopIndexerOps([...singleHop, ...singleHop])).toBeUndefined()
  })
})
