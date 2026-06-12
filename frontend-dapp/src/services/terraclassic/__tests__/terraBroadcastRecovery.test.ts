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
  getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 5n }),
  pollTx: mockPollTx,
  broadcastTx: vi.fn(),
}

beforeEach(() => {
  vi.clearAllMocks()
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
