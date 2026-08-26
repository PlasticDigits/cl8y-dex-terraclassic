import Blockies from 'react-blockies'
import { isValidTerraAddress } from '@/utils/constants'

export type TraderBlockieProps = {
  address: string
  /** CSS pixels. Leaderboard ~18; profile header ~36. Clamped to avoid unbounded canvas scale. */
  size?: number
  className?: string
  'data-testid'?: string
}

const BLOCKIE_CELLS = 6
const SIZE_MIN = 12
const SIZE_MAX = 64

/**
 * Deterministic circular identicon for a trader wallet (GitLab #656).
 * Seed is lowercase bech32 only — never a remote avatar URL. Do not reuse the token logo component.
 */
export function TraderBlockie({
  address,
  size = 18,
  className = '',
  'data-testid': testId = 'trader-identity-blockie',
}: TraderBlockieProps) {
  if (!isValidTerraAddress(address)) return null

  const px = Math.min(SIZE_MAX, Math.max(SIZE_MIN, size))
  const scale = Math.max(2, Math.ceil(px / BLOCKIE_CELLS))
  const seed = address.toLowerCase()
  const wrapClass = `inline-block shrink-0 overflow-hidden rounded-full ${className}`.trim()

  return (
    <span
      className={wrapClass}
      style={{ width: px, height: px }}
      aria-hidden
      data-testid={testId}
      data-blockie-seed={seed}
    >
      <Blockies seed={seed} size={BLOCKIE_CELLS} scale={scale} />
    </span>
  )
}
