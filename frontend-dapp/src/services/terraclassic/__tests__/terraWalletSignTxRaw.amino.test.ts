import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@/utils/terraTxHash', () => ({
  txHashFromTxRaw: vi.fn().mockResolvedValue('TXHASH'),
}))

vi.mock('@goblinhunt/cosmes/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@goblinhunt/cosmes/client')>()
  class FakeTx {
    toStdSignDoc() {
      return {
        chain_id: 'columbus-5',
        fee: { amount: [{ denom: 'uluna', amount: '1' }], gas: '1' },
      }
    }
    toSignDoc() {
      return { bodyBytes: new Uint8Array([1]), authInfoBytes: new Uint8Array([2]) }
    }
    toSignedAmino() {
      return { toBinary: () => new Uint8Array([1]) }
    }
    toSignedDirect() {
      return { toBinary: () => new Uint8Array([2]) }
    }
  }
  return { ...actual, Tx: FakeTx }
})

import { isAtomicWalletConnectPost, signTerraTxRaw, walletUsesAmino } from '../terraWalletSignTxRaw'

const fee = {
  amount: [{ denom: 'uluna', amount: '1' }],
  gasLimit: 1n,
} as never

function keplrWallet(overrides: Record<string, unknown> = {}) {
  const signAmino = vi.fn().mockResolvedValue({
    signed: { fee: { amount: [{ denom: 'uluna', amount: '1' }], gas: '1' }, sequence: '1' },
    signature: { signature: 'aa' },
  })
  const signDirect = vi.fn().mockResolvedValue({
    signed: { bodyBytes: new Uint8Array([1]), authInfoBytes: new Uint8Array([2]) },
    signature: { signature: 'bb' },
  })
  return {
    wallet: {
      id: WalletName.KEPLR,
      type: WalletType.EXTENSION,
      chainId: 'columbus-5',
      address: 'terra1sender',
      pubKey: {},
      getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 1n }),
      ext: { signAmino, signDirect },
      useAmino: false,
      ...overrides,
    },
    signAmino,
    signDirect,
  }
}

describe('isAtomicWalletConnectPost (GitLab #679 residual)', () => {
  it('keeps Station and LuncDash WalletConnect on atomic post', () => {
    expect(
      isAtomicWalletConnectPost({
        id: WalletName.STATION,
        type: WalletType.WALLETCONNECT,
      } as never)
    ).toBe(true)
    expect(
      isAtomicWalletConnectPost({
        id: WalletName.LUNCDASH,
        type: WalletType.WALLETCONNECT,
      } as never)
    ).toBe(true)
    expect(
      isAtomicWalletConnectPost({
        id: WalletName.STATION,
        type: WalletType.EXTENSION,
      } as never)
    ).toBe(false)
    expect(
      isAtomicWalletConnectPost({
        id: WalletName.KEPLR,
        type: WalletType.WALLETCONNECT,
      } as never)
    ).toBe(false)
  })
})

describe('walletUsesAmino (GitLab #567)', () => {
  it('is always true for Station and Cosmostation', () => {
    expect(walletUsesAmino({ id: WalletName.STATION, useAmino: false } as never)).toBe(true)
    expect(walletUsesAmino({ id: WalletName.COSMOSTATION, useAmino: false } as never)).toBe(true)
  })

  it('is true for Keplr when useAmino or isNanoLedger', () => {
    expect(walletUsesAmino({ id: WalletName.KEPLR, useAmino: true } as never)).toBe(true)
    expect(walletUsesAmino({ id: WalletName.KEPLR, useAmino: false, isNanoLedger: true } as never)).toBe(true)
  })

  it('is false for software Keplr', () => {
    expect(walletUsesAmino({ id: WalletName.KEPLR, useAmino: false } as never)).toBe(false)
    expect(walletUsesAmino({ id: WalletName.KEPLR } as never)).toBe(false)
  })
})

describe('signTerraTxRaw amino vs direct (GitLab #567)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('software Keplr uses signDirect with preferNoSetFee', async () => {
    const { wallet, signAmino, signDirect } = keplrWallet({ useAmino: false })
    await signTerraTxRaw(wallet as never, { msgs: [] }, fee)
    expect(signDirect).toHaveBeenCalledTimes(1)
    expect(signAmino).not.toHaveBeenCalled()
    expect(signDirect.mock.calls[0][3]).toEqual({ preferNoSetFee: true, preferNoSetMemo: true })
  })

  it('Keplr useAmino uses signAmino never signDirect', async () => {
    const { wallet, signAmino, signDirect } = keplrWallet({ useAmino: true })
    await signTerraTxRaw(wallet as never, { msgs: [] }, fee)
    expect(signAmino).toHaveBeenCalledTimes(1)
    expect(signDirect).not.toHaveBeenCalled()
    expect(signAmino.mock.calls[0][3]).toEqual({ preferNoSetFee: true, preferNoSetMemo: true })
  })

  it('Keplr isNanoLedger uses signAmino even when useAmino is false', async () => {
    const { wallet, signAmino, signDirect } = keplrWallet({ useAmino: false, isNanoLedger: true })
    await signTerraTxRaw(wallet as never, { msgs: [] }, fee)
    expect(signAmino).toHaveBeenCalledTimes(1)
    expect(signDirect).not.toHaveBeenCalled()
  })

  it('Station always uses signAmino', async () => {
    const { wallet, signAmino, signDirect } = keplrWallet({
      id: WalletName.STATION,
      useAmino: false,
      isNanoLedger: false,
    })
    await signTerraTxRaw(wallet as never, { msgs: [] }, fee)
    expect(signAmino).toHaveBeenCalledTimes(1)
    expect(signDirect).not.toHaveBeenCalled()
  })
})
