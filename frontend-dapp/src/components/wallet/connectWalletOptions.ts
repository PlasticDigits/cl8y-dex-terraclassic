import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'

export type ConnectWalletOption = {
  name: string
  walletName: WalletName
  walletType: WalletType
  connectionLabel: string
}

export type ConnectWalletOptionEnv = {
  isMobileClient: boolean
  keplrInjected: boolean
}

/**
 * Connect list rows (GitLab #554).
 *
 * Mobile Chrome without `window.keplr` offers Keplr via WalletConnect — not an
 * Install-only desktop extension row. Injected Keplr (in-app browser) stays Extension.
 */
export function shouldOfferKeplrWalletConnect(env: ConnectWalletOptionEnv): boolean {
  return env.isMobileClient && !env.keplrInjected
}

export function resolveConnectWalletOptions(env: ConnectWalletOptionEnv): ConnectWalletOption[] {
  const keplrWc = shouldOfferKeplrWalletConnect(env)
  return [
    {
      name: 'Station',
      walletName: WalletName.STATION,
      walletType: WalletType.EXTENSION,
      connectionLabel: 'Extension',
    },
    keplrWc
      ? {
          name: 'Keplr',
          walletName: WalletName.KEPLR,
          walletType: WalletType.WALLETCONNECT,
          connectionLabel: 'WalletConnect',
        }
      : {
          name: 'Keplr',
          walletName: WalletName.KEPLR,
          walletType: WalletType.EXTENSION,
          connectionLabel: 'Extension',
        },
    {
      name: 'Cosmostation',
      walletName: WalletName.COSMOSTATION,
      walletType: WalletType.EXTENSION,
      connectionLabel: 'Extension',
    },
    {
      name: 'LuncDash',
      walletName: WalletName.LUNCDASH,
      walletType: WalletType.WALLETCONNECT,
      connectionLabel: 'WalletConnect',
    },
    {
      name: 'Galaxy Station',
      walletName: WalletName.GALAXYSTATION,
      walletType: WalletType.WALLETCONNECT,
      connectionLabel: 'WalletConnect',
    },
  ]
}
