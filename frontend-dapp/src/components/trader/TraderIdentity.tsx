import type { MouseEventHandler, ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { TraderBlockie } from '@/components/trader/TraderBlockie'
import { isValidTerraAddress } from '@/utils/constants'
import { shortenTraderAddress } from '@/utils/tokenDisplay'

export type TraderIdentityProps = {
  address: string
  /** Compact blockie size (leaderboard). Header uses {@link TraderBlockie} at ~36px beside AddressRow. */
  size?: number
  /** When true and the address is a valid bech32, wrap the chip in `/trader/{full}`. */
  linkToProfile?: boolean
  onClick?: MouseEventHandler<HTMLAnchorElement>
  className?: string
  /** Profile header: pass AddressRow (copy + explorer) instead of the 4/6 text span. */
  children?: ReactNode
  'data-testid'?: string
}

/**
 * Shared trader-as-person chrome: circular blockie + 4/6 label (GitLab #656).
 * Navigation, copy, and explorer always use the full validated bech32.
 */
export function TraderIdentity({
  address,
  size = 18,
  linkToProfile = false,
  onClick,
  className = '',
  children,
  'data-testid': testId = 'trader-identity',
}: TraderIdentityProps) {
  const valid = isValidTerraAddress(address)
  const label = shortenTraderAddress(address)
  const title = valid ? address : undefined
  const rowClass = `inline-flex min-w-0 items-center gap-1.5 whitespace-nowrap ${className}`.trim()

  const inner = (
    <>
      {valid ? <TraderBlockie address={address} size={size} /> : null}
      {children ?? <span className="trader-identity-label min-w-0 font-mono">{label}</span>}
    </>
  )

  if (linkToProfile && valid) {
    return (
      <Link
        to={`/trader/${address}`}
        title={title}
        aria-label={`Trader ${label}`}
        className={`${rowClass} hover:underline`}
        style={{ color: 'var(--mint)' }}
        onClick={onClick}
        data-testid={testId}
      >
        {inner}
      </Link>
    )
  }

  return (
    <span title={title} className={rowClass} data-testid={testId}>
      {inner}
    </span>
  )
}
