import { describe, expect, it } from 'vitest'
import { gasLimitForHybridSwapPublic, getGasLimitForExecuteMsg } from './gas'

describe('getGasLimitForExecuteMsg hybrid parity (GitLab #260, #262)', () => {
  it('shallow book (2 makers) matches dApp scan/park envelope', () => {
    expect(gasLimitForHybridSwapPublic(2)).toBe(1_401_200)
  })

  it('pool-only hybrid uses one-hop pool envelope', () => {
    expect(gasLimitForHybridSwapPublic(0)).toBe(840_000)
  })

  it('4-hop mixed hybrid-first + pool rest matches dApp 6,785,500 (#679)', () => {
    expect(
      getGasLimitForExecuteMsg({
        execute_swap_operations: {
          operations: [
            {
              terra_swap: {
                hybrid: {
                  pool_input: '0',
                  book_input: '10000000000',
                  max_maker_fills: 8,
                },
              },
            },
            { terra_swap: {} },
            { terra_swap: {} },
            { terra_swap: {} },
          ],
        },
      })
    ).toBe(6_785_500)
  })

  it('hops without hybrid do not add 15M (#679)', () => {
    const gas = getGasLimitForExecuteMsg({
      execute_swap_operations: {
        operations: [
          { terra_swap: { hybrid: { pool_input: '0', book_input: '1', max_maker_fills: 8 } } },
          { terra_swap: {} },
        ],
      },
    })
    expect(gas).toBeLessThan(15_000_000)
    expect(gas).not.toBe(1_785_500 + 15_000_000)
  })

  it('send inner swap with shallow book uses same limit as dApp', () => {
    const inner = {
      swap: {
        hybrid: {
          pool_input: '500',
          book_input: '500',
          max_maker_fills: 2,
        },
      },
    }
    const msg = {
      send: {
        msg: Buffer.from(JSON.stringify(inner)).toString('base64'),
      },
    }
    expect(getGasLimitForExecuteMsg(msg)).toBe(1_401_200)
  })
})
