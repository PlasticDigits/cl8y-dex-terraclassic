import { WalletName } from '@goblinhunt/cosmes/wallet'

/** Keplr-compatible `experimentalSuggestChain` surface (Station shim + Keplr family). */
export type KeplrExperimentalSuggest = {
  experimentalSuggestChain?: (chainInfo: Record<string, unknown>) => Promise<void>
  defaultOptions?: { sign?: { preferNoSetFee?: boolean; preferNoSetMemo?: boolean } }
}

/**
 * Resolve the browser extension object that implements `experimentalSuggestChain`.
 * Station exposes this under `window.station.keplr` (GitLab #127: LocalTerra fee/gas step refresh).
 * For **signing** behaviour on LocalTerra + Station, see the cosmes patch (`StationController` amino path, `KeplrExtension` `preferNoSetFee`) and [`skills/AGENTS_TERRACLASSIC_GAS.md`](../../../../skills/AGENTS_TERRACLASSIC_GAS.md).
 */
export function getKeplrLikeExtension(walletName: WalletName): KeplrExperimentalSuggest | undefined {
  if (typeof window === 'undefined') {
    return undefined
  }
  const w = window as unknown as {
    keplr?: KeplrExperimentalSuggest
    cosmostation?: { providers?: { keplr?: KeplrExperimentalSuggest } }
    station?: { keplr?: KeplrExperimentalSuggest }
  }
  switch (walletName) {
    case WalletName.STATION:
      return w.station?.keplr
    case WalletName.KEPLR:
      return w.keplr
    case WalletName.COSMOSTATION:
      return w.cosmostation?.providers?.keplr
    default:
      return undefined
  }
}
