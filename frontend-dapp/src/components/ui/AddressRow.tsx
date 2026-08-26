import { CopyButton } from '@/components/ui/CopyButton'
import { ExplorerLinkIcon } from '@/components/ui/ExplorerLinkIcon'
import { getExplorerAddressUrl, isSafeExplorerHref } from '@/utils/terraExplorer'
import { shortenAddress } from '@/utils/tokenDisplay'

export type AddressRowProps = {
  /** Full bech32 or contract address written to clipboard and used for explorer URL. */
  address: string
  /** When set, show the full address string instead of a shortened label. */
  showFull?: boolean
  /**
   * Keep the label + copy/explorer icons on one row (wallet dropdown header, GitLab #671).
   * Does not change the clipboard / explorer payload. Default wrap stays for Pool / Protocol `showFull`.
   */
  nowrap?: boolean
  /** Passed to `shortenAddress` when `showFull` is false (defaults 8 / 6). */
  startChars?: number
  endChars?: number
  className?: string
  /** Accessible name for copy (e.g. "Copy wallet address"). */
  copyAriaLabel?: string
  /** Accessible name for explorer link (e.g. "View wallet address on explorer"). */
  explorerAriaLabel?: string
  'data-testid'?: string
}

export function AddressRow({
  address,
  showFull = false,
  nowrap = false,
  startChars,
  endChars,
  className = '',
  copyAriaLabel = 'Copy address',
  explorerAriaLabel = 'View address on explorer',
  'data-testid': testId = 'address-row',
}: AddressRowProps) {
  const displayLabel = showFull ? address : shortenAddress(address, startChars ?? 8, endChars ?? 6)
  const explorerUrlRaw = getExplorerAddressUrl(address)
  const explorerUrl = isSafeExplorerHref(explorerUrlRaw) ? explorerUrlRaw : null
  const wrapClass = nowrap ? 'flex-nowrap' : 'flex-wrap'
  const labelOverflow = showFull && !nowrap ? 'break-all' : nowrap ? 'truncate' : ''

  return (
    <span
      className={`address-row inline-flex min-w-0 items-center gap-1 ${wrapClass} ${className}`.trim()}
      data-testid={testId}
    >
      {explorerUrl ? (
        <a
          href={explorerUrl}
          target="_blank"
          rel="noopener noreferrer"
          title={address}
          className={`address-row-label min-w-0 font-mono text-xs underline hover:opacity-80 ${labelOverflow}`.trim()}
        >
          {displayLabel}
        </a>
      ) : (
        <span title={address} className={`address-row-label min-w-0 font-mono text-xs ${labelOverflow}`.trim()}>
          {displayLabel}
        </span>
      )}
      <CopyButton text={address} ariaLabel={copyAriaLabel} data-testid={`${testId}-copy`} />
      {explorerUrl ? (
        <a
          href={explorerUrl}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={explorerAriaLabel}
          className="explorer-link"
          data-testid={`${testId}-explorer`}
        >
          <ExplorerLinkIcon />
        </a>
      ) : null}
    </span>
  )
}
