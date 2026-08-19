import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  TERRA_TX_BROADCAST_TIMEOUT_MESSAGE,
  TERRA_TX_SIGN_STALL_KEPLR_MESSAGE,
  TERRA_TX_SIGN_STALL_LEDGER_MESSAGE,
  TERRA_TX_SIGN_TIMEOUT_MS,
} from '@/utils/terraTxTimeout'
import { resetTerraWalletSignLockForTests } from '../terraWalletSignLock'

const mockSignTerraTxRaw = vi.fn()
const mockRpcBroadcastTx = vi.fn()
const mockPollTx = vi.fn()
const mockPrepareKeplr = vi.fn().mockResolvedValue(undefined)
const mockPrepareStation = vi.fn().mockResolvedValue(undefined)

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

vi.mock('../keplrExtensionConfig', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../keplrExtensionConfig')>()
  return {
    ...actual,
    prepareKeplrExtensionForTerraClassicSign: (...args: unknown[]) => mockPrepareKeplr(...args),
  }
})

vi.mock('../stationExtensionConfig', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../stationExtensionConfig')>()
  return {
    ...actual,
    prepareStationExtensionForTerraClassicSign: (...args: unknown[]) => mockPrepareStation(...args),
  }
})

import { broadcastTerraExecuteContracts } from '../terraBroadcast'

const signed = {
  txRaw: { toBinary: () => new Uint8Array([1]) },
  txHash: 'SIGNEDHASH',
  sequence: 5n,
}

function keplrWallet(overrides: Record<string, unknown> = {}) {
  return {
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
    ...overrides,
  }
}

describe('broadcastTerraExecuteContracts Keplr Ledger (GitLab #567)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetTerraWalletSignLockForTests()
    mockPrepareKeplr.mockResolvedValue(undefined)
    mockPrepareStation.mockResolvedValue(undefined)
    mockSignTerraTxRaw.mockResolvedValue(signed)
    mockRpcBroadcastTx.mockResolvedValue('SIGNEDHASH')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('calls Keplr pre-sign suggest before software Keplr split-path broadcast', async () => {
    await broadcastTerraExecuteContracts(keplrWallet() as never, 'terra1sender', [
      { contract: 'terra1a', msg: { swap: {} } },
    ])

    expect(mockPrepareKeplr).toHaveBeenCalledTimes(1)
    expect(mockPrepareStation).not.toHaveBeenCalled()
    expect(mockRpcBroadcastTx).toHaveBeenCalledTimes(1)
  })

  it('does not call Keplr prepare on Station extension', async () => {
    await broadcastTerraExecuteContracts(keplrWallet({ id: WalletName.STATION }) as never, 'terra1sender', [
      { contract: 'terra1a', msg: { swap: {} } },
    ])
    expect(mockPrepareStation).toHaveBeenCalledTimes(1)
    expect(mockPrepareKeplr).not.toHaveBeenCalled()
  })

  it('sign-stall timeout surfaces Ledger recovery copy, not #173 broadcast copy', async () => {
    vi.useFakeTimers()
    mockSignTerraTxRaw.mockImplementation(() => new Promise(() => {}))

    const pending = broadcastTerraExecuteContracts(keplrWallet({ isNanoLedger: true }) as never, 'terra1sender', [
      { contract: 'terra1a', msg: { swap: {} } },
    ])
    const assertion = expect(pending).rejects.toThrow(TERRA_TX_SIGN_STALL_LEDGER_MESSAGE)
    await vi.advanceTimersByTimeAsync(TERRA_TX_SIGN_TIMEOUT_MS)
    await assertion

    expect(mockRpcBroadcastTx).not.toHaveBeenCalled()
  })

  it('software Keplr sign-stall uses Keplr copy without check-your-connection', async () => {
    vi.useFakeTimers()
    mockSignTerraTxRaw.mockImplementation(() => new Promise(() => {}))

    const pending = broadcastTerraExecuteContracts(keplrWallet({ useAmino: false }) as never, 'terra1sender', [
      { contract: 'terra1a', msg: { swap: {} } },
    ])
    const assertion = expect(pending).rejects.toSatisfy((err: unknown) => {
      const msg = err instanceof Error ? err.message : String(err)
      return (
        msg === TERRA_TX_SIGN_STALL_KEPLR_MESSAGE &&
        !/check your connection/i.test(msg) &&
        msg !== TERRA_TX_BROADCAST_TIMEOUT_MESSAGE
      )
    })
    await vi.advanceTimersByTimeAsync(TERRA_TX_SIGN_TIMEOUT_MS)
    await assertion
    expect(mockRpcBroadcastTx).not.toHaveBeenCalled()
  })

  it('ignores a late signature after sign-stall timeout (no double-broadcast)', async () => {
    vi.useFakeTimers()
    let resolveSign!: (value: typeof signed) => void
    mockSignTerraTxRaw.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveSign = resolve
        })
    )

    const pending = broadcastTerraExecuteContracts(keplrWallet({ isNanoLedger: true }) as never, 'terra1sender', [
      { contract: 'terra1a', msg: { swap: {} } },
    ])
    const assertion = expect(pending).rejects.toThrow(TERRA_TX_SIGN_STALL_LEDGER_MESSAGE)
    await vi.advanceTimersByTimeAsync(TERRA_TX_SIGN_TIMEOUT_MS)
    await assertion

    resolveSign(signed)
    await Promise.resolve()
    await Promise.resolve()
    expect(mockRpcBroadcastTx).not.toHaveBeenCalled()
  })
})
