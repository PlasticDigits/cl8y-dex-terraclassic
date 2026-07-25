import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE } from '@/utils/terraTxTimeout'

const mockSignTerraTxRaw = vi.fn()
const mockRpcBroadcastTx = vi.fn()
const mockPollTx = vi.fn()
const mockPollTxUntilRecoveryDeadline = vi.fn()

vi.mock('../terraWalletSignTxRaw', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../terraWalletSignTxRaw')>()
  return {
    ...actual,
    signTerraTxRaw: (...args: unknown[]) => mockSignTerraTxRaw(...args),
    walletSupportsSplitSignBroadcast: () => true,
    isAtomicWalletConnectPost: () => false,
    bumpWalletCachedSequence: vi.fn(),
  }
})

vi.mock('@goblinhunt/cosmes/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@goblinhunt/cosmes/client')>()
  return {
    ...actual,
    RpcClient: {
      ...actual.RpcClient,
      broadcastTx: (...args: unknown[]) => mockRpcBroadcastTx(...args),
    },
  }
})

vi.mock('../terraTxRecoveryPoll', () => ({
  pollTxUntilRecoveryDeadline: (...args: unknown[]) => mockPollTxUntilRecoveryDeadline(...args),
}))

import { broadcastTerraExecuteContracts } from '../terraBroadcast'

const mockWallet = {
  id: WalletName.KEPLR,
  type: WalletType.EXTENSION,
  chainId: 'columbus-5',
  address: 'terra1sender',
  rpc: 'https://rpc.example',
  pubKey: { toProto: () => ({ key: new Uint8Array(33) }) },
  accountNumber: 1n,
  sequence: 5n,
  getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 5n }),
  pollTx: mockPollTx,
  broadcastTx: vi.fn(),
}

beforeEach(() => {
  vi.clearAllMocks()
  mockWallet.sequence = 5n
  mockSignTerraTxRaw.mockResolvedValue({
    txRaw: { toBinary: () => new Uint8Array([1]) },
    txHash: 'SIGNEDHASH',
    sequence: 5n,
  })
  mockRpcBroadcastTx.mockResolvedValue('SIGNEDHASH')
  mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })
})

describe('broadcastTerraExecuteContracts post-sign recovery (GitLab #359)', () => {
  it('recovers when broadcast RPC times out but tx lands during deadline poll', async () => {
    vi.useFakeTimers()
    mockRpcBroadcastTx.mockImplementation(() => new Promise(() => {}))

    const phases: string[] = []
    const pending = broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { swap: { deadline: Math.floor(Date.now() / 1000) + 60 } } }],
      {
        onPhaseChange: (phase) => {
          phases.push(phase)
        },
      }
    )

    mockPollTxUntilRecoveryDeadline.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const assertion = expect(pending).resolves.toBe('SIGNEDHASH')
    await vi.advanceTimersByTimeAsync(30_000)
    await assertion

    expect(phases).toContain('recovering')
    expect(mockPollTxUntilRecoveryDeadline).toHaveBeenCalledWith(
      'https://rpc.example',
      'SIGNEDHASH',
      expect.any(Number)
    )
    vi.useRealTimers()
  })

  it('only offers retry after recovery poll misses the deadline', async () => {
    vi.useFakeTimers()
    mockRpcBroadcastTx.mockImplementation(() => new Promise(() => {}))
    mockPollTxUntilRecoveryDeadline.mockRejectedValueOnce(new Error(TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE))

    const pending = broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { swap: {} } }],
      {}
    )

    const assertion = expect(pending).rejects.toThrow(TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE)
    await vi.advanceTimersByTimeAsync(30_000)
    await assertion
    vi.useRealTimers()
  })

  it('recovers when broadcast RPC fails with an ambiguous error after sign', async () => {
    mockRpcBroadcastTx.mockRejectedValueOnce(new Error('502 Bad Gateway'))
    mockPollTxUntilRecoveryDeadline.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const phases: string[] = []
    const txHash = await broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { swap: { deadline: Math.floor(Date.now() / 1000) + 120 } } }],
      {
        onPhaseChange: (phase) => {
          phases.push(phase)
        },
      }
    )

    expect(phases).toContain('recovering')
    expect(txHash).toBe('SIGNEDHASH')
    expect(mockPollTxUntilRecoveryDeadline).toHaveBeenCalled()
  })

  it('does not enter recovery on definite CheckTx rejection after sign', async () => {
    // #499 retries once on code-32; both attempts stay definite rejections (no #359 poll).
    mockRpcBroadcastTx
      .mockRejectedValueOnce(new Error('account sequence mismatch, expected 2, got 3'))
      .mockRejectedValueOnce(new Error('account sequence mismatch, expected 2, got 3'))

    const phases: string[] = []
    await expect(
      broadcastTerraExecuteContracts(
        mockWallet as never,
        'terra1sender',
        [{ contract: 'terra1a', msg: { swap: {} } }],
        {
          onPhaseChange: (phase) => {
            phases.push(phase)
          },
        }
      )
    ).rejects.toThrow(/Wallet out of sync/)

    expect(phases).not.toContain('recovering')
    expect(mockPollTxUntilRecoveryDeadline).not.toHaveBeenCalled()
    expect(mockSignTerraTxRaw).toHaveBeenCalledTimes(2)
  })

  it('retries once on account sequence mismatch then succeeds (GitLab #499)', async () => {
    mockRpcBroadcastTx
      .mockRejectedValueOnce(new Error('account sequence mismatch, expected 6, got 5: incorrect account sequence'))
      .mockResolvedValueOnce('SIGNEDHASH2')
    mockSignTerraTxRaw
      .mockResolvedValueOnce({
        txRaw: { toBinary: () => new Uint8Array([1]) },
        txHash: 'SIGNEDHASH',
        sequence: 5n,
      })
      .mockResolvedValueOnce({
        txRaw: { toBinary: () => new Uint8Array([2]) },
        txHash: 'SIGNEDHASH2',
        sequence: 6n,
      })

    const phases: string[] = []
    const txHash = await broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { drip: { token: 'terra1token' } } }],
      {
        onPhaseChange: (phase) => {
          phases.push(phase)
        },
      }
    )

    expect(txHash).toBe('SIGNEDHASH2')
    expect(mockSignTerraTxRaw).toHaveBeenCalledTimes(2)
    expect(mockSignTerraTxRaw.mock.calls[0]?.[3]).toEqual({ useCachedSequence: false })
    expect(mockSignTerraTxRaw.mock.calls[1]?.[3]).toEqual({ useCachedSequence: true })
    expect(mockWallet.sequence).toBe(6n)
    expect(phases).not.toContain('recovering')
    expect(mockPollTxUntilRecoveryDeadline).not.toHaveBeenCalled()
  })

  it('recovers when mempool reports tx already in cache after sign', async () => {
    mockRpcBroadcastTx.mockRejectedValueOnce(new Error('tx already exists in cache'))
    mockPollTxUntilRecoveryDeadline.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const phases: string[] = []
    const txHash = await broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { swap: { deadline: Math.floor(Date.now() / 1000) + 120 } } }],
      {
        onPhaseChange: (phase) => {
          phases.push(phase)
        },
      }
    )

    expect(phases).toContain('recovering')
    expect(txHash).toBe('SIGNEDHASH')
  })

  it('recovers when pollTx times out after broadcast returns hash', async () => {
    vi.useFakeTimers()
    mockPollTx.mockImplementation(() => new Promise(() => {}))
    mockPollTxUntilRecoveryDeadline.mockResolvedValueOnce({
      txResponse: { code: 0, rawLog: '', logs: [] },
    })

    const phases: string[] = []
    const pending = broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { swap: { deadline: Math.floor(Date.now() / 1000) + 120 } } }],
      {
        onPhaseChange: (phase) => {
          phases.push(phase)
        },
      }
    )

    const assertion = expect(pending).resolves.toBe('SIGNEDHASH')
    await vi.advanceTimersByTimeAsync(90_000)
    await assertion

    expect(phases).toContain('recovering')
    vi.useRealTimers()
  })
})
