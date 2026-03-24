import { getNetworkBadgeCopy, getTerraChainLogoPath } from '@/utils/networkDisplay'

export default function NetworkBadge() {
  const { shortLabel, fullLabel, chainId } = getNetworkBadgeCopy()
  const iconSrc = getTerraChainLogoPath(chainId)
  const title = `${fullLabel} · ${chainId}`

  return (
    <div
      className="flex items-center gap-1.5 px-1.5 sm:px-2 py-1 sm:py-1.5 border-2 border-white/25 shrink-0 shadow-[2px_2px_0_#000]"
      style={{ background: 'var(--panel-bg)' }}
      title={title}
      role="region"
      aria-label={title}
    >
      <img src={iconSrc} alt="" className="h-4 w-4 shrink-0 object-contain" aria-hidden />
      <span className="text-[9px] sm:text-xs font-medium uppercase tracking-wide whitespace-nowrap font-heading">
        {shortLabel}
      </span>
    </div>
  )
}
