import { getNetworkBadgeCopy, getTerraChainLogoPath } from '@/utils/networkDisplay'

/**
 * Chain logo + optional short network label for the connected wallet chip trigger.
 * Label is visible from `sm:` up; mobile relies on icon + title and EnvironmentRibbon.
 */
export function WalletChipNetworkIndicator() {
  const { shortLabel, fullLabel, chainId } = getNetworkBadgeCopy()
  const iconSrc = getTerraChainLogoPath(chainId)
  const title = `${fullLabel} · ${chainId}`

  return (
    <div className="wallet-chip-network shrink-0" title={title} data-testid="wallet-chip-network">
      <span className="wallet-trigger-icon">
        <img src={iconSrc} alt="" aria-hidden />
      </span>
      <span className="wallet-chip-network-label hidden sm:inline" data-testid="wallet-network-short-label">
        {shortLabel}
      </span>
    </div>
  )
}
