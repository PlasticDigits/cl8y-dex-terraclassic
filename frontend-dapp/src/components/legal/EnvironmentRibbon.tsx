import { DEFAULT_NETWORK, NETWORKS } from '@/utils/constants'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'

/**
 * Persistent strip under the header so local / testnet / mainnet is obvious even when the wallet badge is crowded (GitLab #138).
 * Desktop/tablet omit the header NetworkBadge and rely on this ribbon as the primary network signal (GitLab #483).
 * Ribbon backgrounds must stay opaque enough that scrolled page copy cannot bleed through (GitLab #482).
 */
export default function EnvironmentRibbon() {
  const { shortLabel, fullLabel, chainId } = getNetworkBadgeCopy()
  const lcdHost = (() => {
    try {
      return new URL(NETWORKS[DEFAULT_NETWORK].terra.lcd).host
    } catch {
      return NETWORKS[DEFAULT_NETWORK].terra.lcd
    }
  })()

  const tone = DEFAULT_NETWORK === 'mainnet' ? 'mainnet' : DEFAULT_NETWORK === 'testnet' ? 'testnet' : 'local'

  const detail =
    DEFAULT_NETWORK === 'local'
      ? `${fullLabel} · ${chainId} · ${lcdHost} (development)`
      : DEFAULT_NETWORK === 'testnet'
        ? `${fullLabel} · ${chainId} · test funds only`
        : `${fullLabel} · ${chainId} · real assets`

  return (
    <div
      className={`app-env-ribbon app-env-ribbon--${tone}`}
      role="status"
      aria-live="polite"
      aria-label={`Environment: ${shortLabel}, chain ${chainId}`}
    >
      <span className="app-env-ribbon-label">{shortLabel}</span>
      <span className="app-env-ribbon-detail">{detail}</span>
    </div>
  )
}
