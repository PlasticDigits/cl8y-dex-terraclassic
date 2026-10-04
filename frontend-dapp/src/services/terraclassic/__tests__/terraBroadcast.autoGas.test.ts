import { afterEach, describe, expect, it, vi } from 'vitest'

import { broadcastTerraExecuteContracts } from '../terraBroadcast'
import { installSwapGasReaderForTests, type SwapGasReadRequest, type SwapGasReader } from '../swapAutoGas'
import { AUTO_GAS_FALLBACK_STEP, AUTO_GAS_STEP_TRANSPORT_LIMIT } from '@/utils/constants'
import { sendHookExecuteSwapOperationsMsg } from '../terraGas'

const mockBroadcastTx = vi.fn()
const mockPollTx = vi.fn()

const mockWallet = {
  address: 'terra1sender',
  broadcastTx: mockBroadcastTx,
  pollTx: mockPollTx,
  getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 1n }),
}

const twoHop = {
  contract: 'terra1token0000000000000000000000000000000001',
  msg: {
    execute_swap_operations: {
      operations: [{ terra_swap: {} }, { terra_swap: {} }],
      max_spread: '0.05',
      minimum_receive: '1',
    },
  },
}

function signedGas(): bigint {
  const fee = mockBroadcastTx.mock.calls.at(-1)?.[1] as { gasLimit?: bigint; amount?: Array<{ denom?: string }> }
  return fee.gasLimit ?? 0n
}

afterEach(() => {
  installSwapGasReaderForTests(null)
  vi.clearAllMocks()
})

describe('broadcastTerraExecuteContracts auto gas (#1360)', () => {
  it('simulates once and signs ceil(gas_used × 1.2) in uluna', async () => {
    const reader = vi.fn(async () => 1_937_976)
    installSwapGasReaderForTests(reader)
    mockBroadcastTx.mockResolvedValue('HASH1')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })

    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [twoHop])

    expect(reader).toHaveBeenCalledTimes(1)
    expect(mockBroadcastTx).toHaveBeenCalledTimes(1)
    expect(signedGas()).toBe(2_325_572n)
    const fee = mockBroadcastTx.mock.calls[0][1] as { amount: Array<{ denom: string }> }
    expect(fee.amount[0].denom).toBe('uluna')
    expect(signedGas()).not.toBe(15_000_000n)
    const unsigned = mockBroadcastTx.mock.calls[0][0] as {
      msgs: Array<{ data: { msg: { execute_swap_operations: { max_spread: string; minimum_receive: string } } } }>
    }
    expect(unsigned.msgs[0].data.msg.execute_swap_operations.max_spread).toBe('0.05')
    expect(unsigned.msgs[0].data.msg.execute_swap_operations.minimum_receive).toBe('1')
  })

  it('steps the query by 200k until simulate succeeds, then signs once', async () => {
    const reader = vi.fn(async (request: SwapGasReadRequest) => {
      const query = request.queryGasLimit ?? 0
      if (query === 1_910_000 + AUTO_GAS_FALLBACK_STEP) return 1_937_976
      return null
    })
    installSwapGasReaderForTests(reader)
    mockBroadcastTx.mockResolvedValue('HASHSTEP')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })

    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [twoHop])

    expect(reader.mock.calls.length).toBeGreaterThan(1)
    expect(mockBroadcastTx).toHaveBeenCalledTimes(1)
    expect(signedGas()).toBe(2_325_572n)
    const queries = reader.mock.calls.map((call) => call[0].queryGasLimit as number)
    expect(queries).toContain(1_910_000 + AUTO_GAS_FALLBACK_STEP)
    expect(queries.some((query) => query > 15_000_000)).toBe(false)
  })

  it('stops after empty simulates and still signs the fallback once, with no code 11 resend', async () => {
    const reader = vi.fn(async () => {
      throw new Error('timeout')
    })
    installSwapGasReaderForTests(reader as SwapGasReader)
    mockBroadcastTx.mockResolvedValue('HASH11')
    mockPollTx.mockResolvedValue({
      txResponse: {
        code: 11,
        rawLog: 'out of gas in location: WriteFlat; gasWanted: 1910000, gasUsed: 1937976',
        logs: [],
      },
    })

    await expect(broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [twoHop])).rejects.toThrow(
      /gas estimate was short/
    )
    expect(reader).toHaveBeenCalledTimes(AUTO_GAS_STEP_TRANSPORT_LIMIT)
    expect(mockBroadcastTx).toHaveBeenCalledTimes(1)
    expect(signedGas()).toBe(1_910_000n)
  })

  it('does not simulate a non-swap execute', async () => {
    const reader = vi.fn(async () => 1_937_976)
    installSwapGasReaderForTests(reader)
    mockBroadcastTx.mockResolvedValue('HASH2')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })

    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
      { contract: 'terra1a', msg: { increase_allowance: { spender: 'terra1p', amount: '1' } } },
    ])
    expect(reader).not.toHaveBeenCalled()
  })

  it('uses the same 1.2× gas for two different contract addresses', async () => {
    installSwapGasReaderForTests(async () => 1_937_976)
    mockBroadcastTx.mockResolvedValue('HASH3')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })

    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
      { contract: 'terra1aaaa000000000000000000000000000000001', msg: twoHop.msg },
    ])
    const first = signedGas()
    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
      { contract: 'terra1bbbb000000000000000000000000000000002', msg: twoHop.msg },
    ])
    expect(signedGas()).toBe(first)
    expect(first).toBe(2_325_572n)
  })

  it('falls back to the wrap + one hop envelope', async () => {
    installSwapGasReaderForTests(async () => null)
    mockBroadcastTx.mockResolvedValue('HASH4')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })

    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
      { contract: 'terra1treasury', msg: { wrap_deposit: {} }, coins: [{ denom: 'uluna', amount: '1' }] },
      { contract: 'terra1wrapped', msg: sendHookExecuteSwapOperationsMsg([{ terra_swap: {} }]) },
    ])
    expect(signedGas()).toBe(1_800_000n)
  })
})
