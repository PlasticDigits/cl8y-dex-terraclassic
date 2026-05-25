import { CopyButton } from '@/components/ui/CopyButton'
import { getExplorerAddressUrl } from '@/utils/terraExplorer'
import { shortenAddress } from '@/utils/tokenDisplay'

export type AddressRowProps = {
  /** Full bech32 or contract address written to clipboard and used for explorer URL. */
  address: string
  /** When set, show the full address string instead of a shortened label. */
  showFull?: boolean
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

const EXPLORER_ICON = (
  <svg className="h-4 w-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
    />
  </svg>
)

export function AddressRow({
  address,
  showFull = false,
  startChars,
  endChars,
  className = '',
  copyAriaLabel = 'Copy address',
  explorerAriaLabel = 'View address on explorer',
  'data-testid': testId = 'address-row',
}: AddressRowProps) {
  const displayLabel = showFull ? address : shortenAddress(address, startChars ?? 8, endChars ?? 6)
  const explorerUrl = getExplorerAddressUrl(address)

  return (
    <span
      className={`address-row inline-flex min-w-0 flex-wrap items-center gap-1 ${className}`.trim()}
      data-testid={testId}
    >
      {explorerUrl ? (
        <a
          href={explorerUrl}
          target="_blank"
          rel="noopener noreferrer"
          title={address}
          className={`address-row-label min-w-0 font-mono text-xs underline hover:opacity-80 ${showFull ? 'break-all' : ''}`.trim()}
        >
          {displayLabel}
        </a>
      ) : (
        <span
          title={address}
          className={`address-row-label min-w-0 font-mono text-xs ${showFull ? 'break-all' : ''}`.trim()}
        >
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
          {EXPLORER_ICON}
        </a>
      ) : null}
    </span>
  )
}
