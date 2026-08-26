import { WalletName } from '@goblinhunt/cosmes/wallet'
import { getKeplrLikeExtension } from '@/services/terraclassic/keplrLikeExtension'

/**
 * Legal next-step hint after connect (GitLab #554 **WC-M12**, [#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658)).
 *
 * The DEX does not implement ADR-036 (**C1**). After WalletConnect there is often
 * no injected signer in the browser tab, so Accept still navigates to the Legal
 * portal. The hint names the DEX Connect set — not Keplr alone.
 *
 * Hide when any keplr-like injector is already present (reuse
 * {@link getKeplrLikeExtension}): `window.keplr`, `window.station?.keplr`, or
 * `window.cosmostation?.providers?.keplr`. `'station' in window` alone is not
 * enough — the Keplr shim may be missing.
 */

/** DEX Connect wallets. Do not include Leap ([#159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)). */
export const LEGAL_TERMS_DEX_WALLET_NAMES = ['Station', 'Keplr', 'Cosmostation', 'Lunc Dash', 'Galaxy Station'] as const

export const LEGAL_TERMS_WALLET_HINT =
  'Sign terms with the same wallet you connected (Station, Keplr, Cosmostation, Lunc Dash, or Galaxy Station).'

/** `useWalletStore.walletType` → retail label for a one-wallet sentence. */
const WALLET_TYPE_HINT_LABEL: Record<string, string> = {
  station: 'Station',
  keplr: 'Keplr',
  cosmostation: 'Cosmostation',
  luncdash: 'Lunc Dash',
  galaxy: 'Galaxy Station',
}

export function shouldShowLegalWalletInAppHint(input: {
  hasSignerInjector: boolean
  signedLatest: boolean | null
}): boolean {
  if (input.hasSignerInjector) return false
  return input.signedLatest === false
}

/** True when Keplr, Station’s keplr shim, or Cosmostation’s keplr provider is injected. */
export function hasLegalSignerInjector(): boolean {
  return Boolean(
    getKeplrLikeExtension(WalletName.KEPLR) ||
    getKeplrLikeExtension(WalletName.STATION) ||
    getKeplrLikeExtension(WalletName.COSMOSTATION)
  )
}

/**
 * Next-step copy: named connected wallet when known, else the DEX Connect list.
 * Never names Leap. Not a link (no wallet-download href).
 */
export function legalTermsWalletHint(walletType?: string | null): string {
  const key = walletType?.trim().toLowerCase() ?? ''
  const label = key ? WALLET_TYPE_HINT_LABEL[key] : undefined
  if (label) {
    return `Open this site in ${label} to accept terms.`
  }
  return LEGAL_TERMS_WALLET_HINT
}
