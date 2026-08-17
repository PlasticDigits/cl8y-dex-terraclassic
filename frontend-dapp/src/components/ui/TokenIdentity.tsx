import { CopyButton } from '@/components/ui/CopyButton'
import { ExplorerLinkIcon } from '@/components/ui/ExplorerLinkIcon'
import { TokenDisplay } from '@/components/ui/TokenDisplay'
import type { AssetInfo } from '@/types'
import { copyPayload, tokenIdentityTarget } from '@/utils/tokenIdentity'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'

export type TokenIdentityProps = {
  info: AssetInfo | null
  /** Factory-stable slot: `base` = asset_0, `quote` = asset_1 (T541-5). */
  role: 'base' | 'quote'
  size?: number
  className?: string
  'data-testid'?: string
}

/**
 * Logo + symbol (text) with sibling copy / explorer — symbol is never an `<a>` (T541-3).
 */
export function TokenIdentity({
  info,
  role,
  size = 16,
  className = '',
  'data-testid': testId = `token-identity-${role}`,
}: TokenIdentityProps) {
  const { displayLabel } = useTokenDisplayInfo(info)
  const target = tokenIdentityTarget(info)
  if (!info || !target) return null

  const copyText = copyPayload(target)
  const symbol = displayLabel || (target.kind === 'cw20' ? 'token' : target.denom)
  const copyAriaLabel = target.kind === 'cw20' ? `Copy ${symbol} address` : `Copy ${symbol} denom`
  const explorerAriaLabel = `View ${symbol} on explorer`
  const explorerUrl = target.kind === 'cw20' ? target.explorerUrl : null

  return (
    <span
      className={`token-identity inline-flex min-w-0 items-center gap-0.5 ${className}`.trim()}
      data-testid={testId}
      data-identity-kind={target.kind}
      data-identity-payload={copyText}
      title={copyText}
    >
      <TokenDisplay info={info} size={size} />
      <CopyButton text={copyText} ariaLabel={copyAriaLabel} data-testid={`${testId}-copy`} />
      {explorerUrl ? (
        <a
          href={explorerUrl}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={explorerAriaLabel}
          title={copyText}
          className="explorer-link"
          data-testid={`${testId}-explorer`}
        >
          <ExplorerLinkIcon />
        </a>
      ) : null}
    </span>
  )
}
