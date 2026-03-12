import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(),
}))

vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn(),
}))

import { queryContract } from '@/services/terraclassic/queries'
import { executeTerraContract } from '@/services/terraclassic/transactions'
import {
  getPairInfo,
  getPool,
  simulateSwap,
  reverseSimulateSwap,
  swap,
  provideLiquidity,
  withdrawLiquidity,
} from '../pair'
import type { AssetInfo, PairInfo, PoolResponse, SimulationResponse, ReverseSimulationResponse } from '@/types'

const mockedQuery = vi.mocked(queryContract)
const mockedExecute = vi.mocked(executeTerraContract)

const PAIR_ADDR = 'terra1paircontract'
const WALLET_ADDR = 'terra1walletaddr'
const TOKEN_A = 'terra1tokena'
const TOKEN_B = 'terra1tokenb'
const LP_TOKEN = 'terra1lptoken'

beforeEach(() => {
  vi.clearAllMocks()
})

describe('getPairInfo', () => {
  it('queries pair info for the given pair address', async () => {
    const pairInfo: PairInfo = {
      asset_infos: [
        { token: { contract_addr: TOKEN_A } },
        { token: { contract_addr: TOKEN_B } },
      ],
      contract_addr: PAIR_ADDR,
      liquidity_token: LP_TOKEN,
    }
    mockedQuery.mockResolvedValueOnce(pairInfo)

    const result = await getPairInfo(PAIR_ADDR)

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, { pair: {} })
    expect(result).toEqual(pairInfo)
  })
})

describe('getPool', () => {
  it('queries pool state for the given pair address', async () => {
    const pool: PoolResponse = {
      assets: [
        { info: { token: { contract_addr: TOKEN_A } }, amount: '1000000' },
        { info: { token: { contract_addr: TOKEN_B } }, amount: '2000000' },
      ],
      total_share: '1414213',
    }
    mockedQuery.mockResolvedValueOnce(pool)

    const result = await getPool(PAIR_ADDR)

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, { pool: {} })
    expect(result).toEqual(pool)
  })
})

describe('simulateSwap', () => {
  it('queries a swap simulation with the correct offer asset', async () => {
    const offerInfo: AssetInfo = { token: { contract_addr: TOKEN_A } }
    const simResp: SimulationResponse = {
      return_amount: '990000',
      spread_amount: '5000',
      commission_amount: '5000',
    }
    mockedQuery.mockResolvedValueOnce(simResp)

    const result = await simulateSwap(PAIR_ADDR, offerInfo, '1000000')

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      simulation: {
        offer_asset: { info: offerInfo, amount: '1000000' },
      },
    })
    expect(result).toEqual(simResp)
  })

  it('works with native token asset info', async () => {
    const offerInfo: AssetInfo = { native_token: { denom: 'uluna' } }
    const simResp: SimulationResponse = {
      return_amount: '500',
      spread_amount: '1',
      commission_amount: '2',
    }
    mockedQuery.mockResolvedValueOnce(simResp)

    await simulateSwap(PAIR_ADDR, offerInfo, '1000')

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      simulation: {
        offer_asset: { info: offerInfo, amount: '1000' },
      },
    })
  })
})

describe('reverseSimulateSwap', () => {
  it('queries a reverse swap simulation with the correct ask asset', async () => {
    const askInfo: AssetInfo = { token: { contract_addr: TOKEN_B } }
    const revSimResp: ReverseSimulationResponse = {
      offer_amount: '1010000',
      spread_amount: '5000',
      commission_amount: '5000',
    }
    mockedQuery.mockResolvedValueOnce(revSimResp)

    const result = await reverseSimulateSwap(PAIR_ADDR, askInfo, '990000')

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      reverse_simulation: {
        ask_asset: { info: askInfo, amount: '990000' },
      },
    })
    expect(result).toEqual(revSimResp)
  })
})

describe('swap', () => {
  it('calls executeTerraContract with CW20 send and base64-encoded swap msg', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_swap')

    const result = await swap(
      WALLET_ADDR,
      TOKEN_A,
      PAIR_ADDR,
      '1000000',
      '1.0',
      '0.01',
      'terra1recipient'
    )

    expect(result).toBe('txhash_swap')
    expect(mockedExecute).toHaveBeenCalledTimes(1)

    const [walletAddr, contractAddr, msg] = mockedExecute.mock.calls[0]
    expect(walletAddr).toBe(WALLET_ADDR)
    expect(contractAddr).toBe(TOKEN_A)
    expect(msg).toHaveProperty('send')

    const sendMsg = (msg as Record<string, unknown>).send as {
      contract: string
      amount: string
      msg: string
    }
    expect(sendMsg.contract).toBe(PAIR_ADDR)
    expect(sendMsg.amount).toBe('1000000')

    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded).toEqual({
      swap: {
        belief_price: '1.0',
        max_spread: '0.01',
        to: 'terra1recipient',
      },
    })
  })

  it('omits optional params when not provided', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_swap2')

    await swap(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '500')

    const sendMsg = (mockedExecute.mock.calls[0][2] as Record<string, unknown>)
      .send as { msg: string }
    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded).toEqual({
      swap: {
        belief_price: undefined,
        max_spread: undefined,
        to: undefined,
      },
    })
  })
})

describe('provideLiquidity', () => {
  it('increases allowance for both tokens then calls provide_liquidity', async () => {
    mockedExecute
      .mockResolvedValueOnce('allowance_a')
      .mockResolvedValueOnce('allowance_b')
      .mockResolvedValueOnce('txhash_provide')

    const result = await provideLiquidity(
      WALLET_ADDR,
      PAIR_ADDR,
      TOKEN_A,
      TOKEN_B,
      '1000',
      '2000'
    )

    expect(result).toBe('txhash_provide')
    expect(mockedExecute).toHaveBeenCalledTimes(3)

    expect(mockedExecute).toHaveBeenNthCalledWith(1, WALLET_ADDR, TOKEN_A, {
      increase_allowance: { spender: PAIR_ADDR, amount: '1000' },
    })
    expect(mockedExecute).toHaveBeenNthCalledWith(2, WALLET_ADDR, TOKEN_B, {
      increase_allowance: { spender: PAIR_ADDR, amount: '2000' },
    })
    expect(mockedExecute).toHaveBeenNthCalledWith(3, WALLET_ADDR, PAIR_ADDR, {
      provide_liquidity: {
        assets: [
          { info: { token: { contract_addr: TOKEN_A } }, amount: '1000' },
          { info: { token: { contract_addr: TOKEN_B } }, amount: '2000' },
        ],
      },
    })
  })

  it('rolls back allowances on provide_liquidity failure', async () => {
    const provideError = new Error('provide_liquidity failed')

    mockedExecute
      .mockResolvedValueOnce('allowance_a')
      .mockResolvedValueOnce('allowance_b')
      .mockRejectedValueOnce(provideError)
      .mockResolvedValueOnce('decrease_a')
      .mockResolvedValueOnce('decrease_b')

    await expect(
      provideLiquidity(WALLET_ADDR, PAIR_ADDR, TOKEN_A, TOKEN_B, '1000', '2000')
    ).rejects.toThrow('provide_liquidity failed')

    expect(mockedExecute).toHaveBeenCalledTimes(5)

    expect(mockedExecute).toHaveBeenNthCalledWith(4, WALLET_ADDR, TOKEN_A, {
      decrease_allowance: { spender: PAIR_ADDR, amount: '1000' },
    })
    expect(mockedExecute).toHaveBeenNthCalledWith(5, WALLET_ADDR, TOKEN_B, {
      decrease_allowance: { spender: PAIR_ADDR, amount: '2000' },
    })
  })

  it('still throws even if rollback decrease_allowance calls fail', async () => {
    const provideError = new Error('provide_liquidity failed')

    mockedExecute
      .mockResolvedValueOnce('allowance_a')
      .mockResolvedValueOnce('allowance_b')
      .mockRejectedValueOnce(provideError)
      .mockRejectedValueOnce(new Error('decrease_a failed'))
      .mockRejectedValueOnce(new Error('decrease_b failed'))

    await expect(
      provideLiquidity(WALLET_ADDR, PAIR_ADDR, TOKEN_A, TOKEN_B, '1000', '2000')
    ).rejects.toThrow('provide_liquidity failed')

    expect(mockedExecute).toHaveBeenCalledTimes(5)
  })
})

describe('withdrawLiquidity', () => {
  it('sends CW20 LP tokens with base64 withdraw_liquidity message', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_withdraw')

    const result = await withdrawLiquidity(WALLET_ADDR, LP_TOKEN, PAIR_ADDR, '500')

    expect(result).toBe('txhash_withdraw')
    expect(mockedExecute).toHaveBeenCalledTimes(1)

    const [walletAddr, contractAddr, msg] = mockedExecute.mock.calls[0]
    expect(walletAddr).toBe(WALLET_ADDR)
    expect(contractAddr).toBe(LP_TOKEN)

    const sendMsg = (msg as Record<string, unknown>).send as {
      contract: string
      amount: string
      msg: string
    }
    expect(sendMsg.contract).toBe(PAIR_ADDR)
    expect(sendMsg.amount).toBe('500')

    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded).toEqual({ withdraw_liquidity: {} })
  })
})
