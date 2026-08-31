import { describe, expect, it } from 'vitest'
import { BASE_GAS_LIMIT, getGasLimitForTx } from '../terraGas'
import { gasLimitForGreedyParams } from '../hybridSwapGas'

function b64(obj: unknown): string {
  return btoa(JSON.stringify(obj))
}

describe('greedy swap gas (GitLab #708 / G13)', () => {
  it('CW20 send + greedy swap uses hybrid book-walk envelope, not 600k pool-only', () => {
    const greedy = { max_maker_fills: 8, book_start_hint: null }
    const gas = getGasLimitForTx({
      send: {
        msg: b64({
          swap: {
            greedy,
            hybrid: null,
          },
        }),
      },
    })
    expect(gas).toBe(gasLimitForGreedyParams(greedy))
    expect(gas).toBeGreaterThan(BASE_GAS_LIMIT)
    expect(gas).toBeGreaterThan(840_000)
  })

  it('router terra_swap.greedy hop is a book walk', () => {
    const greedy = { max_maker_fills: 8 }
    const gas = getGasLimitForTx({
      execute_swap_operations: {
        operations: [
          {
            terra_swap: {
              offer_asset_info: { token: { contract_addr: 'terra1a' } },
              ask_asset_info: { token: { contract_addr: 'terra1b' } },
              greedy,
            },
          },
        ],
      },
    })
    expect(gas).toBe(gasLimitForGreedyParams(greedy))
    expect(gas).toBeGreaterThan(840_000)
  })
})
