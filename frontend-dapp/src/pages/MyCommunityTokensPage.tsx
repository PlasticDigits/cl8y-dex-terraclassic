import { Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getCommunityTokens } from '@/services/indexer/client'
import { isCommunityTaxEnabled } from '@/utils/constants'

export default function MyCommunityTokensPage() {
  const address = useWalletStore((s) => s.address)
  const listQuery = useQuery({
    queryKey: ['communityTokens', address],
    queryFn: () => getCommunityTokens({ manager: address! }),
    enabled: isCommunityTaxEnabled() && !!address,
  })

  if (!isCommunityTaxEnabled()) {
    return (
      <div className="max-w-[640px] mx-auto" data-testid="my-tokens-page">
        <div className="shell-panel-strong py-8 text-center">Create Token is not configured.</div>
      </div>
    )
  }

  if (!address) {
    return (
      <div className="max-w-[640px] mx-auto" data-testid="my-tokens-page">
        <h2 className="text-lg font-semibold uppercase tracking-wide font-heading mb-4">My tokens</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }} data-testid="my-tokens-connect">
          Connect wallet to see tokens you manage.{' '}
          <Link className="underline" to="/token/create">
            Create Token
          </Link>
        </p>
      </div>
    )
  }

  const items = listQuery.data?.items ?? []

  return (
    <div className="max-w-[640px] mx-auto" data-testid="my-tokens-page">
      <h2 className="text-lg font-semibold uppercase tracking-wide font-heading mb-4">My tokens</h2>
      {items.length === 0 ? (
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }} data-testid="my-tokens-empty">
          No community tokens yet.{' '}
          <Link className="underline" to="/token/create">
            Create Token
          </Link>
        </p>
      ) : (
        <ul className="space-y-2">
          {items.map((t) => (
            <li key={t.contract_address} className="shell-panel-strong">
              <Link className="underline break-all" to={`/token/${t.contract_address}/manage`}>
                {t.symbol || t.name || t.contract_address}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
