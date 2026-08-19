import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  prepareKeplrExtensionForTerraClassicSign,
  rememberKeplrNanoLedgerFlag,
  shouldApplyKeplrSignStallTimeout,
  walletIsNanoLedger,
  getSessionNanoLedger,
  setSessionNanoLedger,
} from '../keplrExtensionConfig'

describe('walletIsNanoLedger (GitLab #567)', () => {
  it('is true when isNanoLedger is set', () => {
    expect(
      walletIsNanoLedger({
        id: WalletName.KEPLR,
        type: WalletType.EXTENSION,
        isNanoLedger: true,
      } as never)
    ).toBe(true)
  })

  it('treats Keplr useAmino as Ledger (upstream getKey mapping)', () => {
    expect(
      walletIsNanoLedger({
        id: WalletName.KEPLR,
        type: WalletType.EXTENSION,
        useAmino: true,
      } as never)
    ).toBe(true)
  })

  it('is false for software Keplr', () => {
    expect(
      walletIsNanoLedger({
        id: WalletName.KEPLR,
        type: WalletType.EXTENSION,
        useAmino: false,
      } as never)
    ).toBe(false)
  })

  it('does not treat Station amino as Ledger UX', () => {
    expect(
      walletIsNanoLedger({
        id: WalletName.STATION,
        type: WalletType.EXTENSION,
        useAmino: true,
      } as never)
    ).toBe(false)
  })
})

describe('shouldApplyKeplrSignStallTimeout (GitLab #567)', () => {
  it('applies to Keplr extension including software', () => {
    expect(
      shouldApplyKeplrSignStallTimeout({
        id: WalletName.KEPLR,
        type: WalletType.EXTENSION,
      } as never)
    ).toBe(true)
  })

  it('does not apply to Station extension', () => {
    expect(
      shouldApplyKeplrSignStallTimeout({
        id: WalletName.STATION,
        type: WalletType.EXTENSION,
      } as never)
    ).toBe(false)
  })

  it('does not apply to Keplr WalletConnect', () => {
    expect(
      shouldApplyKeplrSignStallTimeout({
        id: WalletName.KEPLR,
        type: WalletType.WALLETCONNECT,
      } as never)
    ).toBe(false)
  })
})

describe('prepareKeplrExtensionForTerraClassicSign (GitLab #567)', () => {
  beforeEach(() => {
    vi.stubGlobal('window', {} as Window & typeof globalThis)
    setSessionNanoLedger(false)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('suggests Terra Classic chain metadata from getTerraChainSuggestion', async () => {
    const experimentalSuggestChain = vi.fn().mockResolvedValue(undefined)
    const getKey = vi
      .fn()
      .mockResolvedValue({ isNanoLedger: false, name: 'a', bech32Address: 'terra1a', pubKey: new Uint8Array() })
    vi.stubGlobal('window', { keplr: { experimentalSuggestChain, getKey } } as unknown as Window & typeof globalThis)

    await prepareKeplrExtensionForTerraClassicSign({
      id: WalletName.KEPLR,
      type: WalletType.EXTENSION,
      chainId: 'columbus-5',
    } as never)

    expect(experimentalSuggestChain).toHaveBeenCalledTimes(1)
    const info = experimentalSuggestChain.mock.calls[0][0] as { bip44?: { coinType?: number }; chainId?: string }
    expect(info.bip44?.coinType).toBe(330)
    expect(typeof info.chainId).toBe('string')
  })

  it('warns and continues when suggest rejects', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const experimentalSuggestChain = vi.fn().mockRejectedValue(new Error('user closed'))
    vi.stubGlobal('window', { keplr: { experimentalSuggestChain } } as unknown as Window & typeof globalThis)

    await expect(
      prepareKeplrExtensionForTerraClassicSign({
        id: WalletName.KEPLR,
        type: WalletType.EXTENSION,
        chainId: 'columbus-5',
      } as never)
    ).resolves.toBeUndefined()

    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })

  it('no-ops for Station wallets', async () => {
    const experimentalSuggestChain = vi.fn()
    vi.stubGlobal('window', { keplr: { experimentalSuggestChain } } as unknown as Window & typeof globalThis)

    await prepareKeplrExtensionForTerraClassicSign({
      id: WalletName.STATION,
      type: WalletType.EXTENSION,
      chainId: 'columbus-5',
    } as never)

    expect(experimentalSuggestChain).not.toHaveBeenCalled()
  })
})

describe('rememberKeplrNanoLedgerFlag (GitLab #567)', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('stores getKey isNanoLedger on the wallet', async () => {
    const getKey = vi.fn().mockResolvedValue({
      isNanoLedger: true,
      name: 'Ledger',
      bech32Address: 'terra1a',
      pubKey: new Uint8Array(),
    })
    vi.stubGlobal('window', { keplr: { getKey } } as unknown as Window & typeof globalThis)

    const wallet = {
      id: WalletName.KEPLR,
      type: WalletType.EXTENSION,
      chainId: 'columbus-5',
    }
    await rememberKeplrNanoLedgerFlag(wallet as never)
    expect((wallet as { isNanoLedger?: boolean }).isNanoLedger).toBe(true)
    expect(getSessionNanoLedger()).toBe(true)
    expect(getKey).toHaveBeenCalledWith('columbus-5')
  })

  it('does not fail connect when getKey throws', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    vi.stubGlobal('window', {
      keplr: { getKey: vi.fn().mockRejectedValue(new Error('locked')) },
    } as unknown as Window & typeof globalThis)

    await expect(
      rememberKeplrNanoLedgerFlag({
        id: WalletName.KEPLR,
        type: WalletType.EXTENSION,
        chainId: 'columbus-5',
      } as never)
    ).resolves.toBeUndefined()
    warn.mockRestore()
  })
})
