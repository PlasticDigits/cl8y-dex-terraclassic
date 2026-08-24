import { useMemo, useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { TxResultAlert } from '@/components/ui'
import { useWalletStore } from '@/hooks/useWallet'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { getChainContractInfo } from '@/services/terraclassic/queries'
import { isCodeIdWhitelisted } from '@/services/terraclassic/factory'
import {
  migrateAdoptCommunityToken,
  probeHasTaxMap,
  queryCommunityTaxTokenInfo,
  queryLauncherConfig,
} from '@/services/terraclassic/communityTaxToken'
import {
  COMMUNITY_TAX_CODE_ID,
  DEFAULT_NETWORK,
  DOCS_GITLAB_BASE,
  isCommunityTaxEnabled,
  NETWORKS,
} from '@/utils/constants'
import { classifyMigrateSource, MIGRATE_LP_CONFIRM } from '@/utils/communityTaxMigrate'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { sounds } from '@/lib/sounds'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'

export default function MigrateTokenPage() {
  const address = useWalletStore((s) => s.address)
  const [searchParams] = useSearchParams()
  void searchParams.get('manager')
  void searchParams.get('treasury')
  void searchParams.get('payee')
  void searchParams.get('token')
  void searchParams.get('addr')

  const [pasted, setPasted] = useState('')
  const [loaded, setLoaded] = useState('')
  const [txHash, setTxHash] = useState<string | null>(null)

  const chainId = NETWORKS[DEFAULT_NETWORK]?.terra.chainId ?? 'localterra'

  const infoQuery = useQuery({
    queryKey: ['migrateTokenInfo', loaded],
    queryFn: () => getChainContractInfo(loaded),
    enabled: !!loaded,
  })
  const tokenInfoQuery = useQuery({
    queryKey: ['migrateTokenTokenInfo', loaded],
    queryFn: () => queryCommunityTaxTokenInfo(loaded),
    enabled: !!loaded,
  })
  const taxMapQuery = useQuery({
    queryKey: ['migrateTokenTaxMap', loaded],
    queryFn: () => probeHasTaxMap(loaded),
    enabled: !!loaded,
  })
  const whitelistQuery = useQuery({
    queryKey: ['migrateTokenWhitelist', infoQuery.data?.code_id],
    queryFn: () => isCodeIdWhitelisted(infoQuery.data!.code_id),
    enabled: Number.isFinite(infoQuery.data?.code_id),
  })
  const launcherQuery = useQuery({
    queryKey: ['communityTaxLauncherConfig'],
    queryFn: queryLauncherConfig,
    staleTime: 60_000,
    enabled: isCommunityTaxEnabled(),
  })

  const verdict = useMemo(() => {
    if (!loaded || !infoQuery.data || taxMapQuery.data === undefined || !whitelistQuery.data) {
      return null
    }
    return classifyMigrateSource({
      chainId,
      codeId: infoQuery.data.code_id,
      taxCodeId: COMMUNITY_TAX_CODE_ID,
      whitelisted: whitelistQuery.data.whitelisted,
      hasTaxMap: taxMapQuery.data,
      wasmAdmin: infoQuery.data.admin,
      connectedWallet: address,
      tokenAddr: loaded,
    })
  }, [address, chainId, infoQuery.data, loaded, taxMapQuery.data, whitelistQuery.data])

  const migrateMut = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!infoQuery.data || !launcherQuery.data) throw new Error('Token is still loading')
      return migrateAdoptCommunityToken({
        token: loaded,
        wasmAdmin: infoQuery.data.admin,
        factory: launcherQuery.data.factory,
        router: launcherQuery.data.router,
        ust1: launcherQuery.data.ust1,
        cmmTreasury: launcherQuery.data.cmm_treasury,
        sourceCodeId: infoQuery.data.code_id,
      })
    },
    onSuccess: (hash) => {
      sounds.playSuccess()
      setTxHash(hash)
    },
    onError: () => sounds.playError(),
  })

  if (!isCommunityTaxEnabled()) {
    return (
      <div className="max-w-[520px] mx-auto" data-testid="migrate-token-page">
        <div
          className="shell-panel-strong py-8 text-center"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="migrate-token-unavailable"
        >
          Migrate Token is not configured.
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-[520px] mx-auto" data-testid="migrate-token-page">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide font-heading">Migrate Token</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Adopt a listed honest CW20 onto the community tax wasm. Same address, no 50 UST1. Launcher-created tokens stay
          CMM-only.{' '}
          <a
            className="underline"
            href={`${DOCS_GITLAB_BASE}/contracts-terraclassic.md`}
            target="_blank"
            rel="noreferrer"
          >
            Docs
          </a>
        </p>
      </div>

      <div className="shell-panel-strong space-y-4">
        <label className="block">
          <span className="label-glass">Token address</span>
          <input
            className="input-glass w-full"
            value={pasted}
            onChange={(e) => setPasted(e.target.value.trim())}
            data-testid="migrate-token-addr"
            autoComplete="off"
          />
        </label>
        <button
          type="button"
          className="btn-secondary w-full"
          data-testid="migrate-token-load"
          onClick={() => {
            sounds.playButtonPress()
            setTxHash(null)
            if (!isValidTerraBech32Address(pasted)) {
              setLoaded('')
              return
            }
            setLoaded(pasted)
          }}
        >
          Load token
        </button>
        {pasted && !isValidTerraBech32Address(pasted) && (
          <p className="text-xs" style={{ color: 'var(--danger)' }} data-testid="migrate-token-bad-addr">
            Paste a Terra Classic contract address.
          </p>
        )}

        {loaded && (
          <div className="text-sm space-y-2" data-testid="migrate-token-probe">
            {tokenInfoQuery.data && (
              <p>
                {tokenInfoQuery.data.symbol} · {tokenInfoQuery.data.name} · {tokenInfoQuery.data.decimals} dp
              </p>
            )}
            {infoQuery.data && (
              <p style={{ color: 'var(--ink-dim)' }}>
                code {infoQuery.data.code_id} · admin {infoQuery.data.admin}
              </p>
            )}
            {verdict && (
              <p data-testid={`migrate-token-verdict-${verdict.kind}`} style={{ color: 'var(--ink-dim)' }}>
                {verdict.reason}
              </p>
            )}
            {verdict?.kind === 'nogo_8654' && (
              <p data-testid="migrate-token-nogo-8654">
                Not supported. New ticker via <Link to="/token/create">Create Token</Link> or wrap on #558.
              </p>
            )}
          </div>
        )}

        {verdict?.canSubmit && (
          <div className="space-y-3" data-testid="migrate-token-confirm">
            <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
              {MIGRATE_LP_CONFIRM}
            </p>
            <button
              type="button"
              className="btn-primary w-full"
              data-testid="migrate-token-cta"
              disabled={migrateMut.isPending}
              onClick={() => {
                sounds.playButtonPress()
                migrateMut.mutate()
              }}
            >
              {terraBroadcastPendingButtonLabel(migrateMut.phase, migrateMut.isPending, 'Migrate (free)', 'Migrating…')}
            </button>
          </div>
        )}

        {txHash && (
          <div data-testid="migrate-token-success">
            <TxResultAlert hash={txHash} />
            <Link className="underline text-sm" to={`/token/${loaded}/manage`} data-testid="migrate-token-next-manage">
              Manage this token
            </Link>
          </div>
        )}
        {migrateMut.isError && (
          <p className="text-sm" style={{ color: 'var(--danger)' }} data-testid="migrate-token-error">
            {humanizeUserFacingErrorFromUnknown(migrateMut.error)}
          </p>
        )}
      </div>
    </div>
  )
}
