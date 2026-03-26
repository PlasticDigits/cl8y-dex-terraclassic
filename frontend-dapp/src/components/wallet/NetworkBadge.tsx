import { getNetworkBadgeCopy, getTerraChainLogoPath } from '@/utils/networkDisplay'

export default function NetworkBadge() {
  const { shortLabel, fullLabel, chainId } = getNetworkBadgeCopy()
  const iconSrc = getTerraChainLogoPath(chainId)
  const title = `${fullLabel} · ${chainId}`

  return (
    <div className="network-badge" title={title} role="region" aria-label={title}>
      <img src={iconSrc} alt="" aria-hidden />
      <span className="truncate">{shortLabel}</span>
    </div>
  )
}
