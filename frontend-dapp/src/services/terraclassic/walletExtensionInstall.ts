import { WalletName } from '@goblinhunt/cosmes/wallet'
import { getKeplrLikeExtension } from '@/services/terraclassic/keplrLikeExtension'

/**
 * Detects whether a browser extension for this wallet name is injected on `window`.
 * Used by the connect modal so users can see **installed vs not installed** before clicking ([GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139)).
 *
 * **Invariants (must stay aligned with `getKeplrLikeExtension`):**
 * - **Station:** `'station' in window` — same signal as legacy `isStationInstalled` (extension injected; shim may still fail at connect).
 * - **Keplr:** `window.keplr` truthy.
 * - **Leap / Cosmostation:** same objects `getKeplrLikeExtension` reads (`window.leap`, `window.cosmostation?.providers?.keplr`).
 * - **WalletConnect-only names** (e.g. LuncDash, Galaxy Station): returns `true` so the UI does not label them “not installed”.
 */
export function isBrowserWalletExtensionDetected(walletName: WalletName): boolean {
  if (typeof window === 'undefined') {
    return false
  }

  switch (walletName) {
    case WalletName.STATION:
      return 'station' in window
    case WalletName.KEPLR:
      return !!(window as unknown as { keplr?: unknown }).keplr
    case WalletName.LEAP:
    case WalletName.COSMOSTATION:
      return !!getKeplrLikeExtension(walletName)
    case WalletName.LUNCDASH:
    case WalletName.GALAXYSTATION:
      return true
    default:
      return false
  }
}

/** Official download / setup pages (not Chrome IDs) — stable landing URLs for “Install”. */
export const WALLET_EXTENSION_INSTALL_URL: Partial<Record<WalletName, string>> = {
  [WalletName.STATION]: 'https://setup.money/',
  [WalletName.KEPLR]: 'https://www.keplr.app/download',
  [WalletName.LEAP]: 'https://www.leapwallet.io/download',
  [WalletName.COSMOSTATION]: 'https://wallet.cosmostation.io/',
}
