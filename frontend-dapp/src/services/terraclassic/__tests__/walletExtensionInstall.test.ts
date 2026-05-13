import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { WalletName } from '@goblinhunt/cosmes/wallet'

describe('walletExtensionInstall (GitLab #139 connect modal)', () => {
  beforeEach(() => {
    vi.stubGlobal('window', {} as Window & typeof globalThis)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.resetModules()
  })

  it('Station: true when window has station', async () => {
    vi.stubGlobal('window', { station: {} } as unknown as Window & typeof globalThis)
    const { isBrowserWalletExtensionDetected } = await import('../walletExtensionInstall')
    expect(isBrowserWalletExtensionDetected(WalletName.STATION)).toBe(true)
  })

  it('Station: false when station missing', async () => {
    const { isBrowserWalletExtensionDetected } = await import('../walletExtensionInstall')
    expect(isBrowserWalletExtensionDetected(WalletName.STATION)).toBe(false)
  })

  it('Keplr: true when window.keplr set', async () => {
    vi.stubGlobal('window', { keplr: {} } as unknown as Window & typeof globalThis)
    const { isBrowserWalletExtensionDetected } = await import('../walletExtensionInstall')
    expect(isBrowserWalletExtensionDetected(WalletName.KEPLR)).toBe(true)
  })

  it('Cosmostation: true when providers.keplr present', async () => {
    vi.stubGlobal('window', { cosmostation: { providers: { keplr: {} } } } as unknown as Window & typeof globalThis)
    const { isBrowserWalletExtensionDetected } = await import('../walletExtensionInstall')
    expect(isBrowserWalletExtensionDetected(WalletName.COSMOSTATION)).toBe(true)
  })

  it('WalletConnect wallets always report detected (no extension-missing treatment)', async () => {
    const { isBrowserWalletExtensionDetected } = await import('../walletExtensionInstall')
    expect(isBrowserWalletExtensionDetected(WalletName.LUNCDASH)).toBe(true)
    expect(isBrowserWalletExtensionDetected(WalletName.GALAXYSTATION)).toBe(true)
  })

  it('WALLET_EXTENSION_INSTALL_URL covers extension wallets in the modal', async () => {
    const { WALLET_EXTENSION_INSTALL_URL } = await import('../walletExtensionInstall')
    expect(WALLET_EXTENSION_INSTALL_URL[WalletName.STATION]).toMatch(/^https:/)
    expect(WALLET_EXTENSION_INSTALL_URL[WalletName.KEPLR]).toMatch(/^https:/)
    expect(WALLET_EXTENSION_INSTALL_URL[WalletName.COSMOSTATION]).toMatch(/^https:/)
  })
})
