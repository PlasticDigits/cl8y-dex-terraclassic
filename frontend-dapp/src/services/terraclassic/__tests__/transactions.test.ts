import { describe, it, expect, vi, beforeEach } from 'vitest'

const { MockMsgExecuteContract, MockFee } = vi.hoisted(() => {
  const MockMsgExecuteContract = vi.fn(function (
    this: Record<string, unknown>,
    args: Record<string, unknown>
  ) {
    Object.assign(this, { type: 'MsgExecuteContract', ...args })
  })

  const MockFee = vi.fn(function (
    this: Record<string, unknown>,
    args: Record<string, unknown>
  ) {
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
}

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn(),
}))

vi.mock('@goblinhunt/cosmes/client', () => ({
  MsgExecuteContract: MockMsgExecuteContract,
}))

vi.mock('@goblinhunt/cosmes/protobufs', () => ({
  CosmosTxV1beta1Fee: MockFee,
}))

import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { executeTerraContract } from '../transactions'

const mockedGetWallet = vi.mocked(getConnectedWallet)

beforeEach(() => {
  vi.clearAllMocks()
})

describe('executeTerraContract', () => {
  it('throws when wallet is not connected', async () => {
    mockedGetWallet.mockReturnValueOnce(null)

    await expect(
      executeTerraContract('terra1sender', 'terra1contract', { swap: {} })
    ).rejects.toThrow('Wallet not connected')
  })

  it('throws on wallet address mismatch', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)

    await expect(
      executeTerraContract('terra1different', 'terra1contract', { swap: {} })
    ).rejects.toThrow('Wallet address mismatch')
  })

  it('broadcasts and polls a transaction successfully', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValueOnce('ABCD1234')
    mockPollTx.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const result = await executeTerraContract(
      'terra1sender',
      'terra1contract',
      { swap: {} }
    )

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

    await expect(
      executeTerraContract('terra1sender', 'terra1contract', { swap: {} })
    ).rejects.toThrow('out of gas')
  })

  it('wraps user-rejected errors', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockRejectedValueOnce(new Error('User rejected the request'))

    await expect(
      executeTerraContract('terra1sender', 'terra1contract', { swap: {} })
    ).rejects.toThrow('Transaction rejected by user')
  })

  it('wraps network errors', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockRejectedValueOnce(new Error('Failed to fetch'))

    await expect(
      executeTerraContract('terra1sender', 'terra1contract', { swap: {} })
    ).rejects.toThrow('Network error')
  })

  it('wraps unknown string errors', async () => {
    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockRejectedValueOnce('something went wrong')

    await expect(
      executeTerraContract('terra1sender', 'terra1contract', { swap: {} })
    ).rejects.toThrow('Transaction failed')
  })

  it('passes coins to MsgExecuteContract when provided', async () => {
    MockMsgExecuteContract.mockClear()

    mockedGetWallet.mockReturnValueOnce(mockConnectedWallet as never)
    mockBroadcastTx.mockResolvedValueOnce('COINHASH')
    mockPollTx.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const coins = [{ denom: 'uluna', amount: '1000000' }]
    await executeTerraContract(
      'terra1sender',
      'terra1contract',
      { swap: {} },
      coins
    )

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

  it('uses SWAP_GAS_LIMIT for swap messages', async () => {
    const fee = await getFeeForMsg({ swap: {} })
    expect(fee.gasLimit).toBe(BigInt(600000))
  })

  it('uses SWAP_GAS_LIMIT for execute_swap_operations', async () => {
    const fee = await getFeeForMsg({ execute_swap_operations: {} })
    expect(fee.gasLimit).toBe(BigInt(600000))
  })

  it('uses ADD_LIQUIDITY_GAS_LIMIT for provide_liquidity', async () => {
    const fee = await getFeeForMsg({ provide_liquidity: {} })
    expect(fee.gasLimit).toBe(BigInt(500000))
  })

  it('uses REMOVE_LIQUIDITY_GAS_LIMIT for withdraw_liquidity', async () => {
    const fee = await getFeeForMsg({ withdraw_liquidity: {} })
    expect(fee.gasLimit).toBe(BigInt(600000))
  })

  it('uses CREATE_PAIR_GAS_LIMIT for create_pair', async () => {
    const fee = await getFeeForMsg({ create_pair: {} })
    expect(fee.gasLimit).toBe(BigInt(800000))
  })

  it('uses SWAP_GAS_LIMIT for send with inner swap msg', async () => {
    const innerSwap = btoa(JSON.stringify({ swap: {} }))
    const fee = await getFeeForMsg({ send: { msg: innerSwap } })
    expect(fee.gasLimit).toBe(BigInt(600000))
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
})
