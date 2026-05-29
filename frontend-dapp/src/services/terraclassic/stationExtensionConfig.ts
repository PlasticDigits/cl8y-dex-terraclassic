import { getKeplrLikeExtension } from '@/services/terraclassic/keplrLikeExtension'
import { WalletName } from '@goblinhunt/cosmes/wallet'

/**
 * Station exposes a Keplr-compatible shim at `window.station.keplr`.
 * Configure sign options before connect and before every broadcast (GitLab #127, #208).
 */
export function applyStationKeplrShimSignDefaults(): void {
  const stationKeplr = getKeplrLikeExtension(WalletName.STATION)
  if (!stationKeplr) {
    return
  }
  stationKeplr.defaultOptions = {
    sign: {
      preferNoSetFee: true,
      preferNoSetMemo: true,
    },
  }
}
