import { describe, it, expect, vi, beforeEach } from 'vitest'

const { MockMsgExecuteContract, MockFee } = vi.hoisted(() => {
  const MockMsgExecuteContract = vi.fn(function (this: Record<string, unknown>, args: Record<string, unknown>) {
    Object.assign(this, { type: 'MsgExecuteContract', ...args })
  })

  const MockFee = vi.fn(function (this: Record<string, unknown>, args: Record<string, unknown>) {
    Object.assign(this, { type: 'Fee', ...args })
  })

  return { MockMsgExecuteContract, MockFee }
})

const mockBroadcastTx = vi.fn()
const mockPollTx = vi.fn()
const mockConnectedWallet = {
  address: 'terra1sender',
  broadcastTx: mockBroadcastTx,
  pollTx: mockPollTx,
  getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 1n }),
}

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn(),
}))

vi.mock('@goblinhunt/cosmes/client', () => ({
  MsgExecuteContract: MockMsgExecuteContract,
  RpcClient: { broadcastTx: vi.fn().mockResolvedValue('MOCK_TX_HASH') },
}))

vi.mock('@goblinhunt/cosmes/protobufs', () => ({
  CosmosTxV1beta1Fee: MockFee,
}))

import { getConnectedWallet } from '@/services/terraclassic/wallet'
import {
  executeTerraContract,
  executeTerraContractMulti,
  executeCw20AllowanceThen,
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
  estimateMarketPairSwapSequenceUlunaFeesTotal,
  estimateNativeSwapUlunaFeesTotal,
  estimateProvideLiquidityCw20SequenceUlunaFeesTotal,
  estimateProvideLiquidityNativeWrapUlunaFeesTotal,
} from '../transactions'
import { estimateFeeUlunaAmountForGasLimit, getGasLimitForTx } from '../terraGas'

const mockedGetWallet = vi.mocked(getConnectedWallet)

beforeEach(() => {
  vi.clearAllMocks()
})

describe('executeTerraContract', () => {
  it('throws when wallet is not connected', async () => {
    mockedGetWallet.mockReturnValueOnce(null)

    await expect(executeTerraContract('terra1sender', 'terra1contract', { swap: {} })).rejects.toThrow(
      'Wallet not connected'
    )
  })

  it('throws on wallet address mismatch', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)

    await expect(executeTerraContract('terra1different', 'terra1contract', { swap: {} })).rejects.toThrow(
      'Wallet address mismatch'
    )
  })

  it('broadcasts and polls a transaction successfully', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValueOnce('ABCD1234')
    mockPollTx.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const result = await executeTerraContract('terra1sender', 'terra1contract', { swap: {} })

    expect(result).toBe('ABCD1234')
    expect(mockBroadcastTx).toHaveBeenCalledTimes(1)
    expect(mockPollTx).toHaveBeenCalledWith('ABCD1234')
  })

  it('throws when txResponse.code is non-zero', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValueOnce('FAILHASH')
    mockPollTx.mockResolvedValueOnce({
      txResponse: { code: 5, rawLog: 'out of gas', logs: [] },
    })

    // 'out of gas' rawLog is now humanized via tryHumanizeTerraTxMessage (GitLab #134).
    // Assertion updated from raw 'out of gas' to the humanized retail copy.
    await expect(executeTerraContract('terra1sender', 'terra1contract', { swap: {} })).rejects.toThrow(
      'Transaction needed more gas than estimated'
    )
  })

  it('humanizes max spread assertion errors from chain logs', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValueOnce('FAILHASH')
    mockPollTx.mockResolvedValueOnce({
      txResponse: {
        code: 1,
        rawLog:
          'failed to execute message; message index: 0: dispatch: submessages: Max spread assertion: actual spread (0.969444373510098454) exceeds max allowed (0.01): execute wasm contract failed',
        logs: [],
      },
    })

    await expect(executeTerraContract('terra1sender', 'terra1contract', { swap: {} })).rejects.toThrow(
      'Trade rejected: price impact exceeds your slippage protection'
    )
  })

  it('wraps user-rejected errors', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockRejectedValueOnce(new Error('User rejected the request'))

    await expect(executeTerraContract('terra1sender', 'terra1contract', { swap: {} })).rejects.toThrow(
      'Transaction rejected by user'
    )
  })

  it('wraps network errors', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockRejectedValueOnce(new Error('Failed to fetch'))

    await expect(executeTerraContract('terra1sender', 'terra1contract', { swap: {} })).rejects.toThrow('Network error')
  })

  it('rejects when broadcastTx does not settle within the timeout ([GitLab #173])', async () => {
    vi.useFakeTimers()
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockReturnValueOnce(new Promise(() => {}))

    const pending = executeTerraContract('terra1sender', 'terra1contract', { swap: {} })
    const assertion = expect(pending).rejects.toThrow(
      'Could not broadcast the transaction. Check your connection and try again.'
    )
    await vi.advanceTimersByTimeAsync(30_000)
    await assertion
    vi.useRealTimers()
  })

  it('rejects when pollTx does not settle within the timeout ([GitLab #173])', async () => {
    vi.useFakeTimers()
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValueOnce('POLLTIMEOUT')
    mockPollTx.mockReturnValueOnce(new Promise(() => {}))

    const pending = executeTerraContract('terra1sender', 'terra1contract', { swap: {} })
    const assertion = expect(pending).rejects.toThrow(
      'Transaction confirmation timed out. Check your connection and try again.'
    )
    await vi.advanceTimersByTimeAsync(90_000)
    await assertion
    vi.useRealTimers()
  })

  it('wraps unknown string errors', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockRejectedValueOnce('something went wrong')

    await expect(executeTerraContract('terra1sender', 'terra1contract', { swap: {} })).rejects.toThrow(
      'Transaction failed'
    )
  })

  it('passes coins to MsgExecuteContract when provided', async () => {
    MockMsgExecuteContract.mockClear()

    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValueOnce('COINHASH')
    mockPollTx.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const coins = [{ denom: 'uluna', amount: '1000000' }]
    await executeTerraContract('terra1sender', 'terra1contract', { swap: {} }, coins)

    expect(MockMsgExecuteContract).toHaveBeenCalledWith({
      sender: 'terra1sender',
      contract: 'terra1contract',
      msg: { swap: {} },
      funds: coins,
    })
  })
})

describe('gas limit selection (tested indirectly)', () => {
  beforeEach(() => {
    mockedGetWallet.mockReturnValue(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValue('TXHASH')
    mockPollTx.mockResolvedValue({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })
  })

  async function getFeeForMsg(msg: Record<string, unknown>) {
    MockFee.mockClear()

    await executeTerraContract('terra1sender', 'terra1contract', msg)

    return MockFee.mock.calls[0][0] as { gasLimit: bigint }
  }

  it('uses buffered pool-only gas for swap messages (GitLab #115 / #134)', async () => {
    const fee = await getFeeForMsg({ swap: {} })
    expect(fee.gasLimit).toBe(BigInt(840000))
  })

  it('uses router single-hop budget for execute_swap_operations (#353)', async () => {
    const fee = await getFeeForMsg({ execute_swap_operations: { operations: [{ swap: {} }] } })
    expect(fee.gasLimit).toBe(BigInt(1_400_000))
  })

  it('single-hop execute_swap_operations gas stays above #115 observed gasUsed (753_321)', async () => {
    const fee = await getFeeForMsg({ execute_swap_operations: { operations: [{ swap: {} }] } })
    expect(fee.gasLimit).toBeGreaterThan(BigInt(753321))
  })

  it('single-hop execute_swap_operations gas stays above #114 re-verification gasUsed (830_102)', async () => {
    const fee = await getFeeForMsg({ execute_swap_operations: { operations: [{ swap: {} }] } })
    expect(fee.gasLimit).toBeGreaterThan(BigInt(830102))
  })

  it('scales gas by hop count with router floor for multi-hop execute_swap_operations (#353)', async () => {
    const fee = await getFeeForMsg({ execute_swap_operations: { operations: [{ swap: {} }, { swap: {} }] } })
    // floor 950k×2 + safety margin
    expect(fee.gasLimit).toBe(BigInt(1_910_000))
  })

  it('2-hop gas limit stays above live observed out-of-gas usage (1,810,206; #353 reopen)', async () => {
    const fee = await getFeeForMsg({ execute_swap_operations: { operations: [{ swap: {} }, { swap: {} }] } })
    // EMBER->JADE->RUBY OOG'd at gasUsed up to 1,810,206 vs 1,810,000 granted (code 11) — the old
    // ~1,718,000 guard was stale and let the knife-edge 1,810,000 budget through.
    expect(fee.gasLimit).toBeGreaterThan(BigInt(1_810_206))
  })

  it('defaults to 1 hop router budget when operations missing', async () => {
    const fee = await getFeeForMsg({ execute_swap_operations: {} })
    expect(fee.gasLimit).toBe(BigInt(1_400_000))
  })

  it('uses ADD_LIQUIDITY_GAS_LIMIT for provide_liquidity', async () => {
    const fee = await getFeeForMsg({ provide_liquidity: {} })
    expect(fee.gasLimit).toBe(BigInt(650000))
  })

  it('uses REMOVE_LIQUIDITY_GAS_LIMIT for withdraw_liquidity', async () => {
    const fee = await getFeeForMsg({ withdraw_liquidity: {} })
    expect(fee.gasLimit).toBe(BigInt(600000))
  })

  it('uses CREATE_PAIR_GAS_LIMIT for create_pair (#345)', async () => {
    const fee = await getFeeForMsg({ create_pair: {} })
    expect(fee.gasLimit).toBe(BigInt(1_000_000))
  })

  it('uses REGISTER_FEE_DISCOUNT_GAS_LIMIT for register (#384, FT-3)', async () => {
    const fee = await getFeeForMsg({ register: { tier_id: 1 } })
    expect(fee.gasLimit).toBe(BigInt(300_000))
  })

  it('uses DEREGISTER_FEE_DISCOUNT_GAS_LIMIT for deregister (#384, FT-4)', async () => {
    const fee = await getFeeForMsg({ deregister: {} })
    expect(fee.gasLimit).toBe(BigInt(250_000))
  })

  it('uses FAUCET_DRIP_GAS_LIMIT for drip (#474 / #475)', async () => {
    const fee = await getFeeForMsg({ drip: { token: 'terra1token' } })
    expect(fee.gasLimit).toBe(BigInt(400_000))
  })

  it('uses UNWRAP_GAS_LIMIT for send with inner unwrap (#475)', async () => {
    const inner = btoa(JSON.stringify({ unwrap: { recipient: null } }))
    const fee = await getFeeForMsg({ send: { msg: inner } })
    expect(fee.gasLimit).toBe(BigInt(800_000))
  })

  it('adds UNWRAP_GAS_LIMIT when execute_swap_operations has unwrap_output (#343)', async () => {
    const inner = btoa(
      JSON.stringify({
        execute_swap_operations: {
          operations: [{ terra_swap: {} }],
          max_spread: '0.01',
          unwrap_output: true,
        },
      })
    )
    const fee = await getFeeForMsg({ send: { msg: inner } })
    expect(fee.gasLimit).toBeGreaterThanOrEqual(BigInt(1_400_000 + 800_000))
  })

  it('uses buffered pool-only gas for send with inner swap msg (GitLab #134)', async () => {
    const innerSwap = btoa(JSON.stringify({ swap: {} }))
    const fee = await getFeeForMsg({ send: { msg: innerSwap } })
    expect(fee.gasLimit).toBe(BigInt(840000))
  })

  it('uses REMOVE_LIQUIDITY_GAS_LIMIT for send with inner withdraw_liquidity msg', async () => {
    const innerWithdraw = btoa(JSON.stringify({ withdraw_liquidity: {} }))
    const fee = await getFeeForMsg({ send: { msg: innerWithdraw } })
    expect(fee.gasLimit).toBe(BigInt(600000))
  })

  it('uses BASE_GAS_LIMIT for increase_allowance', async () => {
    const fee = await getFeeForMsg({ increase_allowance: {} })
    expect(fee.gasLimit).toBe(BigInt(200000))
  })

  it('uses BASE_GAS_LIMIT for decrease_allowance', async () => {
    const fee = await getFeeForMsg({ decrease_allowance: {} })
    expect(fee.gasLimit).toBe(BigInt(200000))
  })

  it('uses BASE_GAS_LIMIT for unknown messages', async () => {
    const fee = await getFeeForMsg({ unknown_action: {} })
    expect(fee.gasLimit).toBe(BigInt(200000))
  })

  it('uses SWAP_GAS_LIMIT for send without inner msg', async () => {
    const fee = await getFeeForMsg({ send: {} })
    expect(fee.gasLimit).toBe(BigInt(600000))
  })

  it('uses SWAP_GAS_LIMIT for send with invalid base64 msg', async () => {
    const fee = await getFeeForMsg({ send: { msg: '!!!invalid!!!' } })
    expect(fee.gasLimit).toBe(BigInt(600000))
  })

  it('uses WRAP_GAS_LIMIT for wrap_deposit messages (#353)', async () => {
    const fee = await getFeeForMsg({ wrap_deposit: {} })
    expect(fee.gasLimit).toBe(BigInt(400_000))
  })

  it('scales gas for send with inner execute_swap_operations', async () => {
    const innerMsg = btoa(
      JSON.stringify({ execute_swap_operations: { operations: [{ swap: {} }, { swap: {} }, { swap: {} }] } })
    )
    const fee = await getFeeForMsg({ send: { msg: innerMsg } })
    // 3 hops × floor 950k + safety margin
    expect(fee.gasLimit).toBe(BigInt(2_860_000))
  })

  it('uses CANCEL_LIMIT_ORDER_GAS_LIMIT for cancel_limit_order', async () => {
    const fee = await getFeeForMsg({ cancel_limit_order: { order_id: 1 } })
    expect(fee.gasLimit).toBe(BigInt(450000))
  })

  it('uses CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT for claim_expired_limit_order', async () => {
    const fee = await getFeeForMsg({ claim_expired_limit_order: { order_id: 2 } })
    expect(fee.gasLimit).toBe(BigInt(450000))
  })

  it('uses batch gas for place_limit_order_batch by rung count', async () => {
    const fee = await getFeeForMsg({
      place_limit_order_batch: {
        side: 'bid',
        orders: [{ price: '1', amount: '100', max_adjust_steps: 32 }],
      },
    })
    expect(fee.gasLimit).toBe(BigInt(580000))
  })

  it('uses batch gas for send with inner place_limit_order_batch', async () => {
    const inner = btoa(
      JSON.stringify({
        place_limit_order_batch: {
          side: 'bid',
          orders: [{ price: '1', amount: '100', max_adjust_steps: 32 }],
        },
      })
    )
    const fee = await getFeeForMsg({ send: { msg: inner } })
    expect(fee.gasLimit).toBe(BigInt(580000))
  })

  it('uses quote-driven hybrid gas for send with inner swap (deep book cap)', async () => {
    const inner = btoa(
      JSON.stringify({
        swap: {
          max_spread: '0.01',
          hybrid: {
            pool_input: '0',
            book_input: '1000',
            max_maker_fills: 8,
            book_start_hint: null,
          },
        },
      })
    )
    const fee = await getFeeForMsg({ send: { msg: inner } })
    expect(fee.gasLimit).toBe(BigInt(1_785_500))
  })

  it('shallow-book hybrid send budgets 500-step scan worst case (GitLab #262)', async () => {
    const inner = btoa(
      JSON.stringify({
        swap: {
          hybrid: {
            pool_input: '500',
            book_input: '500',
            max_maker_fills: 2,
            book_start_hint: null,
          },
        },
      })
    )
    const fee = await getFeeForMsg({ send: { msg: inner } })
    expect(fee.gasLimit).toBe(BigInt(1_401_200))
  })

  it('bumps execute_swap_operations gas when a hop includes hybrid', async () => {
    const fee = await getFeeForMsg({
      execute_swap_operations: {
        operations: [
          {
            terra_swap: {
              offer_asset_info: { token: { contract_addr: 'terra1a' } },
              ask_asset_info: { token: { contract_addr: 'terra1b' } },
              hybrid: {
                pool_input: '1',
                book_input: '2',
                max_maker_fills: 4,
              },
            },
          },
        ],
      },
    })
    expect(fee.gasLimit).toBe(BigInt(1_529_300))
  })

  it('2-hop execute_swap_operations with hybrid on each hop sums per-hop quote-driven gas', async () => {
    const fee = await getFeeForMsg({
      execute_swap_operations: {
        operations: [
          {
            terra_swap: {
              offer_asset_info: { token: { contract_addr: 'terra1a' } },
              ask_asset_info: { token: { contract_addr: 'terra1b' } },
              hybrid: { pool_input: '1', book_input: '1', max_maker_fills: 4 },
            },
          },
          {
            terra_swap: {
              offer_asset_info: { token: { contract_addr: 'terra1b' } },
              ask_asset_info: { token: { contract_addr: 'terra1c' } },
              hybrid: { pool_input: '1', book_input: '1', max_maker_fills: 4 },
            },
          },
        ],
      },
    })
    expect(fee.gasLimit).toBe(BigInt(3_058_600))
  })
})

describe('executeCw20AllowanceThen', () => {
  beforeEach(() => {
    mockedGetWallet.mockReturnValue(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValue('ALLOWHASH')
    mockPollTx.mockResolvedValue({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })
  })

  it('broadcasts increase_allowance then runs the follow-up action', async () => {
    const followUp = vi.fn().mockResolvedValue('PLACEHASH')

    const result = await executeCw20AllowanceThen('terra1sender', 'terra1token', 'terra1pair', '1000', followUp)

    expect(result).toBe('PLACEHASH')
    expect(mockBroadcastTx).toHaveBeenCalledTimes(1)
    expect(followUp).toHaveBeenCalledTimes(1)
    expect(MockMsgExecuteContract).toHaveBeenCalledWith(
      expect.objectContaining({
        contract: 'terra1token',
        msg: { increase_allowance: { spender: 'terra1pair', amount: '1000' } },
      })
    )
  })
})

describe('estimateLimitOrderPlaceSequenceUlunaFeesTotal', () => {
  it('sums fee uluna for increase_allowance + batch place gas at effective gas price', () => {
    const total = estimateLimitOrderPlaceSequenceUlunaFeesTotal(1)
    // allowance 200k + batch 580k gas × 28.325 uluna (GitLab #206)
    expect(total).toBe(22_093_500n)
  })
})

describe('estimateMarketPairSwapSequenceUlunaFeesTotal', () => {
  it('sums allowance + pool-only pair swap gas when hybrid is off', () => {
    const total = estimateMarketPairSwapSequenceUlunaFeesTotal(false)
    // 200k × 28.325 + 840k × 28.325 = 29_458_000 uluna (GitLab #134 buffered pool-only swap + safety margin)
    expect(total).toBe(29_458_000n)
  })

  it('sums allowance + hybrid pair swap gas when hybrid is on', () => {
    const total = estimateMarketPairSwapSequenceUlunaFeesTotal(true)
    // 200k allowance + ~1.7855M hybrid (8 makers, 500 scan steps) × 28.325
    expect(total).toBe(56_239_288n)
  })
})

describe('estimateProvideLiquidityCw20SequenceUlunaFeesTotal', () => {
  it('sums fee uluna for two increase_allowance + provide_liquidity gas limits at effective gas price', () => {
    const total = estimateProvideLiquidityCw20SequenceUlunaFeesTotal()
    // 2×(200k × 28.325) + 650k × 28.325 = 29_741_250 uluna (ADD_LIQUIDITY_GAS_LIMIT)
    expect(total).toBe(29_741_250n)
  })
})

describe('estimateNativeSwapUlunaFeesTotal (GitLab #213)', () => {
  it('uses wrap_deposit gas for direct wrap', () => {
    const total = estimateNativeSwapUlunaFeesTotal({ isDirectWrap: true, needsWrapInput: false })
    expect(total).toBe(estimateFeeUlunaAmountForGasLimit(getGasLimitForTx({ wrap_deposit: {} })))
  })

  it('sums wrap + router send gas for native input swap', () => {
    const total = estimateNativeSwapUlunaFeesTotal({ isDirectWrap: false, needsWrapInput: true, hopCount: 1 })
    expect(total).toBeGreaterThan(estimateFeeUlunaAmountForGasLimit(getGasLimitForTx({ wrap_deposit: {} })))
  })
})

describe('estimateProvideLiquidityNativeWrapUlunaFeesTotal (GitLab #213)', () => {
  it('scales with wrap_deposit count in combined tx', () => {
    const one = estimateProvideLiquidityNativeWrapUlunaFeesTotal(1)
    const two = estimateProvideLiquidityNativeWrapUlunaFeesTotal(2)
    expect(two).toBeGreaterThan(one)
  })
})

describe('executeTerraContractMulti', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockedGetWallet.mockReturnValue(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValue('MULTIHASH')
    mockPollTx.mockResolvedValue({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })
  })

  it('builds a TX with multiple MsgExecuteContract messages', async () => {
    MockMsgExecuteContract.mockClear()

    await executeTerraContractMulti('terra1sender', [
      {
        contract: 'terra1treasury',
        msg: { wrap_deposit: {} },
        coins: [{ denom: 'uluna', amount: '1000000' }],
      },
      {
        contract: 'terra1token',
        msg: { send: { contract: 'terra1router', amount: '1000000', msg: btoa('{}') } },
      },
    ])

    expect(MockMsgExecuteContract).toHaveBeenCalledTimes(2)
    expect(MockMsgExecuteContract).toHaveBeenNthCalledWith(1, {
      sender: 'terra1sender',
      contract: 'terra1treasury',
      msg: { wrap_deposit: {} },
      funds: [{ denom: 'uluna', amount: '1000000' }],
    })
    expect(MockMsgExecuteContract).toHaveBeenNthCalledWith(2, {
      sender: 'terra1sender',
      contract: 'terra1token',
      msg: { send: { contract: 'terra1router', amount: '1000000', msg: btoa('{}') } },
      funds: [],
    })
  })

  it('sums gas across all messages', async () => {
    MockFee.mockClear()

    await executeTerraContractMulti('terra1sender', [
      { contract: 'terra1treasury', msg: { wrap_deposit: {} } },
      { contract: 'terra1token', msg: { swap: {} } },
    ])

    const feeCall = MockFee.mock.calls[0][0] as { gasLimit: bigint }
    expect(feeCall.gasLimit).toBe(BigInt(400_000 + 840_000))
  })

  it('throws when wallet is not connected', async () => {
    mockedGetWallet.mockReturnValueOnce(null)

    await expect(
      executeTerraContractMulti('terra1sender', [{ contract: 'terra1c', msg: { swap: {} } }])
    ).rejects.toThrow('Wallet not connected')
  })

  it('throws on wallet address mismatch', async () => {
    await expect(
      executeTerraContractMulti('terra1different', [{ contract: 'terra1c', msg: { swap: {} } }])
    ).rejects.toThrow('Wallet address mismatch')
  })
})
