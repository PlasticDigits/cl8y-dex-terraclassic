import { describe, expect, it } from 'vitest'
import { gasLimitForHybridSwapPublic, getGasLimitForExecuteMsg } from './gas'

describe('getGasLimitForExecuteMsg hybrid parity (GitLab #260, #262)', () => {
  it('shallow book (2 makers) matches dApp scan/park envelope', () => {
    expect(gasLimitForHybridSwapPublic(2)).toBe(1_401_200)
  })

  it('pool-only hybrid uses one-hop pool envelope', () => {
    expect(gasLimitForHybridSwapPublic(0)).toBe(840_000)
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
