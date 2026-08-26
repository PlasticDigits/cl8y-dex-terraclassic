import { AddressRow } from '@/components/ui/AddressRow'
import { TokenIdentity } from '@/components/ui/TokenIdentity'
import type { AssetInfo } from '@/types'
import { formatPairV2LpUsd } from '@/utils/formatProtocolStats'
import { isPairIdentityAddress, pairIdentityLegOrder } from '@/utils/tokenIdentity'

export type PairTokenLinksProps = {
  pairAddress: string | null | undefined
  asset0: AssetInfo | null | undefined
  asset1: AssetInfo | null | undefined
  /** #524 display invert — reorders chips only; payloads stay factory assets. */
  inverted?: boolean
  /**
   * Optional factory v2 AMM LP USD from `GET /api/v1/pairs/{addr}` `liquidity_usd` (#664).
   * Do **not** pass this on `/pool` table rows (#655 owns that column).
   */
  liquidityUsd?: string | null
  className?: string
  'data-testid'?: string
}

/**
 * Compact pair-leg + pair-contract identity row (GitLab #541).
 * Optional **v2 LP** USD chip on Trade / Charts only (GitLab #664).
 * Hidden when the pair bech32 is missing or invalid (T541-6).
 */
export function PairTokenLinks({
  pairAddress,
  asset0,
  asset1,
  inverted = false,
  liquidityUsd,
  className = '',
  'data-testid': testId = 'pair-token-links',
}: PairTokenLinksProps) {
  const pair = pairAddress?.trim() ?? ''
  if (!isPairIdentityAddress(pair)) return null

  const order = pairIdentityLegOrder(inverted)
  const lpLabel = formatPairV2LpUsd(liquidityUsd)

  return (
    <div
      className={`pair-token-links flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 ${className}`.trim()}
      data-testid={testId}
      data-pair-addr={pair}
    >
      {order.map((role) => {
        const info = role === 'base' ? asset0 : asset1
        if (!info) return null
        return <TokenIdentity key={role} info={info} role={role} />
      })}
      <AddressRow
        address={pair}
        startChars={8}
        endChars={6}
        copyAriaLabel="Copy pair address"
        explorerAriaLabel="View pair on explorer"
        data-testid="token-identity-pair"
      />
      {lpLabel ? (
        <span
          className="token-identity-v2-lp-usd inline-flex min-w-0 items-baseline gap-1 text-xs"
          data-testid="token-identity-v2-lp-usd"
          title="Factory v2 pool USD (reference)."
        >
          <span className="token-identity-v2-lp-usd-label font-semibold" style={{ color: 'var(--ink-dim)' }}>
            v2 LP
          </span>
          <span className="token-identity-v2-lp-usd-value font-mono tabular-nums" style={{ color: 'var(--ink)' }}>
            {lpLabel}
          </span>
        </span>
      ) : null}
    </div>
  )
}
