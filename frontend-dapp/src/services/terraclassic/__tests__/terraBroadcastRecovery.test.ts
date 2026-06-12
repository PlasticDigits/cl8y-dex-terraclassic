import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import * as CosmesClient from '@goblinhunt/cosmes/client'
import type { CosmosTxV1beta1TxRaw as ProtoTxRaw } from 'cosmes/protobufs'
import { broadcastTerraExecuteContracts } from '../terraBroadcast'
import { terraRecoveryDeadlineUnixFromEntries, terraRecoveryPollDeadlineUnix } from '../terraMsgDeadline'
import { pollTerraTxRecovery } from '../terraTxRecoveryPoll'
import { installSignedTxHashCapture, txHashFromTxRaw } from '../terraWalletSignTxRaw'
import { TERRA_TX_BROADCAST_TIMEOUT_MESSAGE } from '@/utils/terraTxTimeout'

type RpcBroadcastFn = (endpoint: string, txRaw: ProtoTxRaw) => Promise<string>

function mockTxRaw(bytes: number[]): ProtoTxRaw {
  return { toBinary: () => new Uint8Array(bytes) } as ProtoTxRaw
}

const mockBroadcastTx = vi.fn()
const mockPollTx = vi.fn()

const mockWallet = {
  id: 'mnemonic',
  type: 'mnemonic',
  address: 'terra1sender',
  rpc: 'http://127.0.0.1:26657',
  broadcastTx: mockBroadcastTx,
  pollTx: mockPollTx,
}

beforeEach(() => {
  vi.clearAllMocks()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('terraRecoveryDeadlineUnixFromEntries', () => {
  it('reads top-level swap deadline', () => {
    expect(
      terraRecoveryDeadlineUnixFromEntries([{ contract: 'terra1p', msg: { swap: { deadline: 1_700_000_000 } } }])
    ).toBe(1_700_000_000)
  })

  it('reads nested CW20 send hook deadline', () => {
    expect(
      terraRecoveryDeadlineUnixFromEntries([
        {
          contract: 'terra1cw20',
          msg: { send: { contract: 'terra1p', amount: '1', msg: { swap: { deadline: 1_800_000_000 } } } },
        },
      ])
    ).toBe(1_800_000_000)
  })
})

describe('txHashFromTxRaw / installSignedTxHashCapture', () => {
  it('derives uppercase hex hash from TxRaw bytes', () => {
    expect(txHashFromTxRaw(mockTxRaw([1, 2, 3, 4, 5]))).toMatch(/^[0-9A-F]{64}$/)
  })

  it('captures hash when RpcClient.broadcastTx is invoked', async () => {
    const rpc = (CosmesClient as { RpcClient: { broadcastTx: RpcBroadcastFn } }).RpcClient
    const txRaw = mockTxRaw([9, 8, 7])
    const expected = txHashFromTxRaw(txRaw)
    const original = rpc.broadcastTx.bind(rpc)
    rpc.broadcastTx = vi.fn(async (_endpoint, raw) => txHashFromTxRaw(raw))
    const capture = installSignedTxHashCapture()
    try {
      await rpc.broadcastTx('http://rpc', txRaw)
      expect(capture.signedTxHash).toBe(expected)
    } finally {
      capture.restore()
      rpc.broadcastTx = original
    }
  })
})

describe('pollTerraTxRecovery', () => {
  it('resolves when pollTx eventually returns code 0', async () => {
    vi.useFakeTimers()
    mockPollTx
      .mockRejectedValueOnce(new Error('Tx not found'))
      .mockResolvedValueOnce({ txResponse: { code: 0, rawLog: '', logs: [] } })

    const pending = pollTerraTxRecovery(mockWallet as never, 'HASH1', Math.floor(Date.now() / 1000) + 30)
    await vi.advanceTimersByTimeAsync(2_500)
    await expect(pending).resolves.toBeUndefined()
    expect(mockPollTx).toHaveBeenCalledTimes(2)
  })
})

describe('broadcastTerraExecuteContracts post-sign recovery (GitLab #359 / #368)', () => {
  it('enters recovering and polls when broadcast times out after sign', async () => {
    vi.useFakeTimers()
    const phases: string[] = []
    const rpc = (CosmesClient as { RpcClient: { broadcastTx: RpcBroadcastFn } }).RpcClient
    const txRaw = mockTxRaw([1, 2, 3])
    const signedHash = txHashFromTxRaw(txRaw)
    const originalBroadcast = rpc.broadcastTx.bind(rpc)

    mockBroadcastTx.mockImplementation(async () => {
      await rpc.broadcastTx('http://rpc', txRaw)
      return signedHash
    })
    rpc.broadcastTx = vi.fn(() => new Promise(() => {})) as RpcBroadcastFn

    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })

    const pending = broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { swap: { deadline: Math.floor(Date.now() / 1000) + 600 } } }],
      { onPhaseChange: (phase) => phases.push(phase) }
    )

    await vi.advanceTimersByTimeAsync(30_000)
    expect(phases).toContain('recovering')
    await vi.advanceTimersByTimeAsync(5_000)
    await expect(pending).resolves.toBe(signedHash)
    expect(mockPollTx).toHaveBeenCalledWith(signedHash, expect.any(Object))
    rpc.broadcastTx = originalBroadcast
  })

  it('does not enter recovery when signing fails before broadcast', async () => {
    const phases: string[] = []
    mockBroadcastTx.mockRejectedValueOnce(new Error('User rejected the request'))

    await expect(
      broadcastTerraExecuteContracts(
        mockWallet as never,
        'terra1sender',
        [{ contract: 'terra1a', msg: { swap: {} } }],
        { onPhaseChange: (phase) => phases.push(phase) }
      )
    ).rejects.toThrow(/rejected by user/i)

    expect(phases).not.toContain('recovering')
    expect(mockPollTx).not.toHaveBeenCalled()
  })

  it('keeps pre-sign broadcast timeout copy when no signed hash exists', async () => {
    vi.useFakeTimers()
    mockBroadcastTx.mockReturnValueOnce(new Promise(() => {}))

    const pending = broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
      { contract: 'terra1a', msg: { swap: {} } },
    ])
    const assertion = expect(pending).rejects.toThrow(TERRA_TX_BROADCAST_TIMEOUT_MESSAGE)
    await vi.advanceTimersByTimeAsync(30_000)
    await assertion
  })

  it('uses msg deadline for recovery poll window', () => {
    const future = Math.floor(Date.now() / 1000) + 120
    const deadline = terraRecoveryPollDeadlineUnix([{ contract: 'terra1a', msg: { swap: { deadline: future } } }])
    expect(deadline).toBe(future)
  })
})
