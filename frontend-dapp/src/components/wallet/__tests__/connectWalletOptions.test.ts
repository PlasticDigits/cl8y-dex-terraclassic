import { describe, expect, it } from 'vitest'
import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { resolveConnectWalletOptions, shouldOfferKeplrWalletConnect } from '../connectWalletOptions'

describe('resolveConnectWalletOptions (GitLab #554)', () => {
  it('offers Keplr WalletConnect on mobile when window.keplr is absent', () => {
    expect(shouldOfferKeplrWalletConnect({ isMobileClient: true, keplrInjected: false })).toBe(true)
    const keplr = resolveConnectWalletOptions({ isMobileClient: true, keplrInjected: false }).find(
      (row) => row.walletName === WalletName.KEPLR
    )
    expect(keplr?.walletType).toBe(WalletType.WALLETCONNECT)
    expect(keplr?.connectionLabel).toBe('WalletConnect')
  })

  it('keeps Keplr Extension when injected (in-app browser, WC-M7)', () => {
    expect(shouldOfferKeplrWalletConnect({ isMobileClient: true, keplrInjected: true })).toBe(false)
    const keplr = resolveConnectWalletOptions({ isMobileClient: true, keplrInjected: true }).find(
      (row) => row.walletName === WalletName.KEPLR
    )
    expect(keplr?.walletType).toBe(WalletType.EXTENSION)
  })

  it('keeps desktop Keplr as Extension even without injection', () => {
    const keplr = resolveConnectWalletOptions({ isMobileClient: false, keplrInjected: false }).find(
      (row) => row.walletName === WalletName.KEPLR
    )
    expect(keplr?.walletType).toBe(WalletType.EXTENSION)
  })
})
