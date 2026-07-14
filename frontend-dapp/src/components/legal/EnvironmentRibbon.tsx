import { DEFAULT_NETWORK, NETWORKS } from '@/utils/constants'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'

/**
 * Network / environment strip in the app footer on all breakpoints (GitLab #138).
 * Desktop/tablet omit the header NetworkBadge for density (#483); wallet chip + this footer strip carry network context.
 * Tint layers panel-bg so the strip remains readable on both themes (#482 opacity intent, footer placement).
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
