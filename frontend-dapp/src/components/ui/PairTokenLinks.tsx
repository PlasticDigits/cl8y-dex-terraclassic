import { AddressRow } from '@/components/ui/AddressRow'
import { TokenIdentity } from '@/components/ui/TokenIdentity'
import type { AssetInfo } from '@/types'
import { isPairIdentityAddress, pairIdentityLegOrder } from '@/utils/tokenIdentity'

export type PairTokenLinksProps = {
  pairAddress: string | null | undefined
  asset0: AssetInfo | null | undefined
  asset1: AssetInfo | null | undefined
  /** #524 display invert — reorders chips only; payloads stay factory assets. */
  inverted?: boolean
  className?: string
  'data-testid'?: string
}

/**
 * Compact pair-leg + pair-contract identity row (GitLab #541).
 * Hidden when the pair bech32 is missing or invalid (T541-6).
 */
export function PairTokenLinks({
  pairAddress,
  asset0,
  asset1,
  inverted = false,
  className = '',
  'data-testid': testId = 'pair-token-links',
}: PairTokenLinksProps) {
  const pair = pairAddress?.trim() ?? ''
  if (!isPairIdentityAddress(pair)) return null

  const order = pairIdentityLegOrder(inverted)

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
    </div>
  )
}
