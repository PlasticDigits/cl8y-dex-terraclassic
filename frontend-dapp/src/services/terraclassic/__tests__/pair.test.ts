import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(),
}))

vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn(),
  executeTerraContractMulti: vi.fn(),
  executeCw20AllowanceThen: vi.fn(),
}))

import { queryContract } from '@/services/terraclassic/queries'
import {
  executeTerraContract,
  executeTerraContractMulti,
  executeCw20AllowanceThen,
} from '@/services/terraclassic/transactions'
import {
  getPairInfo,
  getPool,
  getPairPaused,
  getAssetCodeIds,
  simulateSwap,
  reverseSimulateSwap,
  simulateHybridSwap,
  swap,
  placeLimitOrderWithAllowance,
  updateLimitOrderPrice,
  provideLiquidity,
  withdrawLiquidity,
  claimExpiredLimitOrder,
  cancelLimitOrder,
  queryOrderStatus,
  parsePairOrderStatus,
} from '../pair'
import type {
  AssetInfo,
  HybridReverseSimulationResponse,
  HybridSimulationResponse,
  PairInfo,
  PairPausedResponse,
  PoolResponse,
} from '@/types'

const mockedQuery = vi.mocked(queryContract)
const mockedExecute = vi.mocked(executeTerraContract)
const mockedExecuteMulti = vi.mocked(executeTerraContractMulti)
const mockedAllowanceThen = vi.mocked(executeCw20AllowanceThen)

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
      asset_infos: [{ token: { contract_addr: TOKEN_A } }, { token: { contract_addr: TOKEN_B } }],
      contract_addr: PAIR_ADDR,
      liquidity_token: LP_TOKEN,
    }
    mockedQuery.mockResolvedValueOnce(pairInfo)

    const result = await getPairInfo(PAIR_ADDR)

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, { pair: {} })
    expect(result).toEqual(pairInfo)
  })
})

describe('getPairPaused', () => {
  it('queries is_paused on the pair contract', async () => {
    const body: PairPausedResponse = { paused: false }
    mockedQuery.mockResolvedValueOnce(body)

    const result = await getPairPaused(PAIR_ADDR)

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, { is_paused: {} })
    expect(result).toEqual(body)
  })
})

describe('getAssetCodeIds', () => {
  it('queries get_asset_code_ids on the pair contract', async () => {
    mockedQuery.mockResolvedValueOnce({ code_ids: [10184, 6036] })

    const result = await getAssetCodeIds(PAIR_ADDR)

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, { get_asset_code_ids: {} })
    expect(result).toEqual({ code_ids: [10184, 6036] })
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
    const simResp: HybridSimulationResponse = {
      return_amount: '990000',
      spread_amount: '5000',
      commission_amount: '5000',
      book_return_amount: '0',
      pool_return_amount: '990000',
    }
    mockedQuery.mockResolvedValueOnce(simResp)

    const result = await simulateSwap(PAIR_ADDR, offerInfo, '1000000')

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      hybrid_simulation: {
        offer_asset: { info: offerInfo, amount: '1000000' },
        hybrid: {
          pool_input: '1000000',
          book_input: '0',
          max_maker_fills: 1,
          book_start_hint: undefined,
        },
      },
    })
    expect(result).toEqual(simResp)
  })

  it('works with native token asset info', async () => {
    const offerInfo: AssetInfo = { native_token: { denom: 'uluna' } }
    const simResp: HybridSimulationResponse = {
      return_amount: '500',
      spread_amount: '1',
      commission_amount: '2',
      book_return_amount: '0',
      pool_return_amount: '500',
    }
    mockedQuery.mockResolvedValueOnce(simResp)

    await simulateSwap(PAIR_ADDR, offerInfo, '1000')

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      hybrid_simulation: {
        offer_asset: { info: offerInfo, amount: '1000' },
        hybrid: {
          pool_input: '1000',
          book_input: '0',
          max_maker_fills: 1,
          book_start_hint: undefined,
        },
      },
    })
  })

  it('passes optional trader on hybrid_simulation when wallet connected (GitLab #245)', async () => {
    const offerInfo: AssetInfo = { token: { contract_addr: TOKEN_A } }
    const simResp: HybridSimulationResponse = {
      return_amount: '995000',
      spread_amount: '0',
      commission_amount: '5000',
      book_return_amount: '0',
      pool_return_amount: '995000',
    }
    mockedQuery.mockResolvedValueOnce(simResp)

    await simulateSwap(PAIR_ADDR, offerInfo, '1000000', { trader: WALLET_ADDR })

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      hybrid_simulation: {
        offer_asset: { info: offerInfo, amount: '1000000' },
        hybrid: {
          pool_input: '1000000',
          book_input: '0',
          max_maker_fills: 1,
          book_start_hint: undefined,
        },
        trader: WALLET_ADDR,
      },
    })
  })
})

describe('simulateHybridSwap', () => {
  it('omits trader when wallet disconnected', async () => {
    const offerInfo: AssetInfo = { token: { contract_addr: TOKEN_A } }
    const hybrid = { pool_input: '500', book_input: '500', max_maker_fills: 8, book_start_hint: null }
    mockedQuery.mockResolvedValueOnce({
      return_amount: '990',
      spread_amount: '0',
      commission_amount: '0',
      book_return_amount: '0',
      pool_return_amount: '990',
    })

    await simulateHybridSwap(PAIR_ADDR, offerInfo, '1000', hybrid)

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      hybrid_simulation: {
        offer_asset: { info: offerInfo, amount: '1000' },
        hybrid: {
          pool_input: '500',
          book_input: '500',
          max_maker_fills: 8,
          book_start_hint: undefined,
        },
      },
    })
  })
})

describe('reverseSimulateSwap', () => {
  it('queries a reverse swap simulation with the correct ask asset', async () => {
    const askInfo: AssetInfo = { token: { contract_addr: TOKEN_B } }
    const revSimResp: HybridReverseSimulationResponse = {
      offer_amount: '1010000',
      spread_amount: '5000',
      commission_amount: '5000',
      book_return_amount: '0',
      pool_return_amount: '990000',
    }
    mockedQuery.mockResolvedValueOnce(revSimResp)

    const result = await reverseSimulateSwap(PAIR_ADDR, askInfo, '990000')

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, {
      hybrid_reverse_simulation: {
        ask_asset: { info: askInfo, amount: '990000' },
        hybrid: {
          pool_input: '1',
          book_input: '0',
          max_maker_fills: 1,
          book_start_hint: null,
        },
      },
    })
    expect(result).toEqual(revSimResp)
  })
})

describe('placeLimitOrderWithAllowance', () => {
  it('routes through executeCw20AllowanceThen with batch hook (GitLab #127 / #206)', async () => {
    mockedAllowanceThen.mockImplementationOnce(async (_w, _t, _s, _a, run) => run())
    mockedExecute.mockResolvedValueOnce('txhash_limit')

    const result = await placeLimitOrderWithAllowance(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '500000', 'bid', '1.5', 3, null)

    expect(result).toBe('txhash_limit')
    expect(mockedAllowanceThen).toHaveBeenCalledWith(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '500000', expect.any(Function))
    const sendMsg = (mockedExecute.mock.calls[0][2] as { send: { msg: string } }).send
    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded.place_limit_order_batch).toMatchObject({
      side: 'bid',
      orders: [{ price: '1.5', amount: '500000', max_adjust_steps: 3 }],
    })
  })

  it('encodes optional hint_after_order_id on batch item (GitLab #261)', async () => {
    mockedAllowanceThen.mockImplementationOnce(async (_w, _t, _s, _a, run) => run())
    mockedExecute.mockResolvedValueOnce('txhash_limit_hint')

    await placeLimitOrderWithAllowance(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '500000', 'bid', '0.95', 16, null, 42)

    const sendMsg = (mockedExecute.mock.calls[0][2] as { send: { msg: string } }).send
    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded.place_limit_order_batch.orders[0]).toMatchObject({
      price: '0.95',
      hint_after_order_id: 42,
      max_adjust_steps: 16,
    })
  })

  it('scales human UST1/USTR price to raw on-chain units (GitLab #529)', async () => {
    mockedAllowanceThen.mockImplementationOnce(async (_w, _t, _s, _a, run) => run())
    mockedExecute.mockResolvedValueOnce('txhash_limit_529')

    await placeLimitOrderWithAllowance(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '1000000', 'ask', '78.76', 32, null, null, {
      decimals0: 6,
      decimals1: 18,
    })

    const sendMsg = (mockedExecute.mock.calls[0][2] as { send: { msg: string } }).send
    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded.place_limit_order_batch.orders[0].price).toBe('78760000000000')
  })
})

describe('updateLimitOrderPrice', () => {
  it('calls executeTerraContract with update_limit_order_price (GitLab #247)', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_update_price')

    const result = await updateLimitOrderPrice(WALLET_ADDR, PAIR_ADDR, 7, '1.25', 32, 6)

    expect(result).toBe('txhash_update_price')
    expect(mockedExecute).toHaveBeenCalledWith(WALLET_ADDR, PAIR_ADDR, {
      update_limit_order_price: {
        order_id: 7,
        price: '1.25',
        hint_after_order_id: 6,
        max_adjust_steps: 32,
      },
    })
  })
})

describe('swap', () => {
  it('calls executeTerraContract with CW20 send and base64-encoded swap msg', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_swap')

    const result = await swap(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '1000000', '1.0', '0.01', 'terra1recipient')

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

    const sendMsg = (mockedExecute.mock.calls[0][2] as Record<string, unknown>).send as { msg: string }
    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded).toEqual({
      swap: {
        belief_price: undefined,
        max_spread: undefined,
        to: undefined,
      },
    })
  })

  it('includes hybrid params in CW20 swap msg (SwapPage direct pair path)', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_hybrid')

    await swap(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '1000', undefined, undefined, undefined, {
      hybrid: {
        pool_input: '600',
        book_input: '400',
        max_maker_fills: 8,
        book_start_hint: 42,
      },
    })

    const sendMsg = (mockedExecute.mock.calls[0][2] as Record<string, unknown>).send as { msg: string }
    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded).toEqual({
      swap: {
        belief_price: undefined,
        max_spread: undefined,
        to: undefined,
        hybrid: {
          pool_input: '600',
          book_input: '400',
          max_maker_fills: 8,
          book_start_hint: 42,
        },
      },
    })
  })

  it('omits book_start_hint in swap msg when hybrid hint is null', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_hybrid2')

    await swap(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '1000', undefined, undefined, undefined, {
      hybrid: {
        pool_input: '1000',
        book_input: '0',
        max_maker_fills: 8,
        book_start_hint: null,
      },
    })

    const sendMsg = (mockedExecute.mock.calls[0][2] as Record<string, unknown>).send as { msg: string }
    const decoded = JSON.parse(atob(sendMsg.msg)) as {
      swap: { hybrid?: { book_start_hint?: number } }
    }
    expect(decoded.swap.hybrid).toBeDefined()
    expect(decoded.swap.hybrid?.book_start_hint).toBeUndefined()
  })

  it('includes deadline and trader with hybrid in swap msg', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_hybrid3')

    await swap(WALLET_ADDR, TOKEN_A, PAIR_ADDR, '2000', '2.0', '0.05', 'terra1recv', {
      hybrid: {
        pool_input: '1000',
        book_input: '1000',
        max_maker_fills: 4,
        book_start_hint: null,
      },
      deadline: 1_700_000_000,
      trader: 'terra1traderreg',
    })

    const sendMsg = (mockedExecute.mock.calls[0][2] as Record<string, unknown>).send as { msg: string }
    const decoded = JSON.parse(atob(sendMsg.msg))
    expect(decoded).toEqual({
      swap: {
        belief_price: '2.0',
        max_spread: '0.05',
        to: 'terra1recv',
        deadline: 1_700_000_000,
        trader: 'terra1traderreg',
        hybrid: {
          pool_input: '1000',
          book_input: '1000',
          max_maker_fills: 4,
        },
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

    const result = await provideLiquidity(WALLET_ADDR, PAIR_ADDR, TOKEN_A, TOKEN_B, '1000', '2000')

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

  it('rolls back allowances on provide_liquidity failure via one multi-msg tx', async () => {
    const provideError = new Error('provide_liquidity failed')

    mockedExecute
      .mockResolvedValueOnce('allowance_a')
      .mockResolvedValueOnce('allowance_b')
      .mockRejectedValueOnce(provideError)
    mockedExecuteMulti.mockResolvedValueOnce('rollback_multi')

    await expect(provideLiquidity(WALLET_ADDR, PAIR_ADDR, TOKEN_A, TOKEN_B, '1000', '2000')).rejects.toThrow(
      'provide_liquidity failed'
    )

    expect(mockedExecute).toHaveBeenCalledTimes(3)
    expect(mockedExecuteMulti).toHaveBeenCalledTimes(1)
    expect(mockedExecuteMulti).toHaveBeenCalledWith(WALLET_ADDR, [
      {
        contract: TOKEN_A,
        msg: { decrease_allowance: { spender: PAIR_ADDR, amount: '1000' } },
      },
      {
        contract: TOKEN_B,
        msg: { decrease_allowance: { spender: PAIR_ADDR, amount: '2000' } },
      },
    ])
  })

  it('still throws original error if rollback multi-tx fails', async () => {
    const provideError = new Error('provide_liquidity failed')

    mockedExecute
      .mockResolvedValueOnce('allowance_a')
      .mockResolvedValueOnce('allowance_b')
      .mockRejectedValueOnce(provideError)
    mockedExecuteMulti.mockRejectedValueOnce(new Error('rollback multi failed'))

    await expect(provideLiquidity(WALLET_ADDR, PAIR_ADDR, TOKEN_A, TOKEN_B, '1000', '2000')).rejects.toThrow(
      'provide_liquidity failed'
    )

    expect(mockedExecute).toHaveBeenCalledTimes(3)
    expect(mockedExecuteMulti).toHaveBeenCalledTimes(1)
  })
})

describe('claimExpiredLimitOrder', () => {
  it('calls executeTerraContract with claim_expired_limit_order on the pair', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_claim')

    const result = await claimExpiredLimitOrder(WALLET_ADDR, PAIR_ADDR, 99)

    expect(result).toBe('txhash_claim')
    expect(mockedExecute).toHaveBeenCalledWith(WALLET_ADDR, PAIR_ADDR, {
      claim_expired_limit_order: { order_id: 99 },
    })
  })
})

describe('queryOrderStatus / cancelLimitOrder (GitLab #530)', () => {
  it('queries OrderStatus with factory order_id only (invert-safe)', async () => {
    mockedQuery.mockResolvedValueOnce({
      order_id: 1,
      status: 'active',
      owner: WALLET_ADDR,
      side: 'ask',
      price: '82.044004487226',
      remaining: '1000000',
    })

    const result = await queryOrderStatus(PAIR_ADDR, 1)

    expect(mockedQuery).toHaveBeenCalledWith(PAIR_ADDR, { order_status: { order_id: 1 } })
    expect(result.status).toBe('active')
    expect(parsePairOrderStatus(result)).toBe('active')
  })

  it('rejects order_id 0 before LCD (L21)', async () => {
    await expect(queryOrderStatus(PAIR_ADDR, 0)).rejects.toThrow(/Invalid order id/)
    expect(mockedQuery).not.toHaveBeenCalled()
  })

  it('does not coerce a missing status string to unknown', () => {
    expect(parsePairOrderStatus({ order_id: 1, status: 'bogus' })).toBeUndefined()
    expect(parsePairOrderStatus(undefined)).toBeUndefined()
  })

  it('cancel payload is pair + order_id only (no price / invert fields)', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_cancel')
    const result = await cancelLimitOrder(WALLET_ADDR, PAIR_ADDR, 1)
    expect(result).toBe('txhash_cancel')
    expect(mockedExecute).toHaveBeenCalledWith(WALLET_ADDR, PAIR_ADDR, {
      cancel_limit_order: { order_id: 1 },
    })
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
