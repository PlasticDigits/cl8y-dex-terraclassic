import { useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { PayWithAnyToken } from '@/components/payments/PayWithAnyToken'
import { TxResultAlert } from '@/components/ui'
import { useWalletStore } from '@/hooks/useWallet'
import { getTokens } from '@/services/indexer/client'
import { getChainContractInfo } from '@/services/terraclassic/queries'
import {
  mintCommunityTax,
  queryCommunityTaxConfig,
  queryCommunityTaxFeatures,
  skimAutoLp,
  type CommunityTaxFeaturesResponse,
} from '@/services/terraclassic/communityTaxToken'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import {
  CMM_GOVERNANCE_ADDR,
  COMMUNITY_TAX_CODE_ID,
  COMMUNITY_TOKEN_LAUNCHER,
  isCommunityTaxEnabled,
  UST1_TOKEN_ADDRESS,
} from '@/utils/constants'
import {
  COMMUNITY_TAX_SKUS,
  isUnlockableAfterCreate,
  parseTaxBps,
  type CommunityTaxSkuId,
} from '@/utils/communityTaxSku'
import {
  buildEnableFeatureInvoice,
  buildSettingsBatchInvoice,
  settingsBatchIsEmpty,
  type SettingsBatchFields,
} from '@/utils/communityTaxInvoice'
import { isManagerWallet } from '@/utils/communityTaxManager'
import { getTerraAddressInputError, isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { sounds } from '@/lib/sounds'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'

export default function ManageTokenPage() {
  const { addr = '' } = useParams<{ addr: string }>()
  const address = useWalletStore((s) => s.address)
  const [buyBps, setBuyBps] = useState('')
  const [sellBps, setSellBps] = useState('')
  const [treasury, setTreasury] = useState('')
  const [transferBps, setTransferBps] = useState('')
  const [exemptAdd, setExemptAdd] = useState('')
  const [mintTo, setMintTo] = useState('')
  const [mintAmount, setMintAmount] = useState('')
  const [revokeAck, setRevokeAck] = useState(false)
  const [saveHash, setSaveHash] = useState<string | null>(null)
  const [unlockSku, setUnlockSku] = useState<CommunityTaxSkuId | ''>('')

  const tokenOk = isValidTerraBech32Address(addr)

  const infoQuery = useQuery({
    queryKey: ['communityTaxInfo', addr],
    queryFn: () => getChainContractInfo(addr),
    enabled: isCommunityTaxEnabled() && tokenOk,
  })
  const configQuery = useQuery({
    queryKey: ['communityTaxConfig', addr],
    queryFn: () => queryCommunityTaxConfig(addr),
    enabled: isCommunityTaxEnabled() && tokenOk,
  })
  const featuresQuery = useQuery({
    queryKey: ['communityTaxFeatures', addr],
    queryFn: () => queryCommunityTaxFeatures(addr),
    enabled: isCommunityTaxEnabled() && tokenOk,
  })

  const cfg = configQuery.data
  const feats = featuresQuery.data
  const isManager = isManagerWallet(address, cfg?.manager)
  const codeMismatch = infoQuery.data && COMMUNITY_TAX_CODE_ID > 0 && infoQuery.data.code_id !== COMMUNITY_TAX_CODE_ID
  const unverifiedAdmin =
    infoQuery.data?.admin &&
    CMM_GOVERNANCE_ADDR &&
    infoQuery.data.admin.toLowerCase() !== CMM_GOVERNANCE_ADDR.toLowerCase()

  const buy = parseTaxBps(buyBps || String(cfg?.buy_bps ?? 0))
  const sell = parseTaxBps(sellBps || String(cfg?.sell_bps ?? 0))
  const transfer = parseTaxBps(transferBps || String(cfg?.transfer_bps ?? 0))
  const treasuryDraft = treasury.trim() || cfg?.treasury || ''
  const treasuryErr = treasuryDraft ? getTerraAddressInputError(treasuryDraft) : null

  const settings: SettingsBatchFields = {}
  if (cfg && buy.ok && buy.bps !== cfg.buy_bps) settings.buy_bps = buy.bps
  if (cfg && sell.ok && sell.bps !== cfg.sell_bps) settings.sell_bps = sell.bps
  if (cfg && treasuryDraft && treasuryDraft !== cfg.treasury && !treasuryErr) settings.treasury = treasuryDraft
  if (feats?.transfer_tax && cfg && transfer.ok && transfer.bps !== cfg.transfer_bps) {
    settings.transfer_bps = transfer.bps
  }
  if (feats?.exemption_directory && exemptAdd.trim() && !getTerraAddressInputError(exemptAdd.trim())) {
    settings.add_exempt = [exemptAdd.trim()]
  }
  if (revokeAck && feats?.mint_control && !cfg?.mint_revoked) settings.revoke_mint = true

  const transferDirtyLocked = Boolean(transferBps.trim()) && !feats?.transfer_tax
  const saveReady =
    isManager && !settingsBatchIsEmpty(settings) && !transferDirtyLocked && buy.ok && sell.ok && !treasuryErr

  const saveInvoice =
    saveReady && UST1_TOKEN_ADDRESS
      ? buildSettingsBatchInvoice({ token: addr, ust1: UST1_TOKEN_ADDRESS, settings })
      : null

  const unlockInvoice =
    unlockSku && isUnlockableAfterCreate(unlockSku) && COMMUNITY_TOKEN_LAUNCHER && UST1_TOKEN_ADDRESS
      ? buildEnableFeatureInvoice({
          launcher: COMMUNITY_TOKEN_LAUNCHER,
          ust1: UST1_TOKEN_ADDRESS,
          token: addr,
          sku: unlockSku,
        })
      : null

  const tokensQuery = useQuery({
    queryKey: ['indexerTokens'],
    queryFn: getTokens,
    staleTime: 60_000,
    enabled: isCommunityTaxEnabled(),
  })
  const pickerTokens = useMemo(() => {
    const fromIdx = tokensQuery.data?.map((t) => t.contract_address).filter((a): a is string => !!a) ?? []
    return [...new Set([UST1_TOKEN_ADDRESS, ...fromIdx].filter(Boolean))]
  }, [tokensQuery.data])

  const mintMut = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect wallet')
      if (getTerraAddressInputError(mintTo)) throw new Error('Invalid mint recipient')
      return mintCommunityTax(address, addr, mintTo.trim(), mintAmount.trim())
    },
    onSuccess: () => sounds.playSuccess(),
    onError: () => sounds.playError(),
  })
  const skimMut = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect wallet')
      if (!cfg?.autolp) throw new Error('AutoLP is not bound')
      return skimAutoLp(address, cfg.autolp)
    },
    onSuccess: () => sounds.playSuccess(),
    onError: () => sounds.playError(),
  })

  if (!isCommunityTaxEnabled()) {
    return (
      <div className="max-w-[640px] mx-auto" data-testid="manage-token-page">
        <div className="shell-panel-strong py-8 text-center" data-testid="manage-token-unavailable">
          Create Token is not configured.
        </div>
      </div>
    )
  }

  if (!tokenOk) {
    return (
      <div className="max-w-[640px] mx-auto" data-testid="manage-token-page">
        <p className="text-sm" style={{ color: 'var(--danger)' }}>
          Invalid token address.
        </p>
      </div>
    )
  }

  if (codeMismatch) {
    return (
      <div className="max-w-[640px] mx-auto" data-testid="manage-token-page">
        <div className="shell-panel-strong" data-testid="manage-token-wrong-template">
          This contract is not the community tax template (code {infoQuery.data?.code_id}). Tax SKUs are hidden.
        </div>
      </div>
    )
  }

  if (configQuery.isError) {
    return (
      <div className="max-w-[640px] mx-auto" data-testid="manage-token-page">
        <p className="text-sm" style={{ color: 'var(--danger)' }} data-testid="manage-token-query-error">
          {humanizeUserFacingErrorFromUnknown(configQuery.error)}
        </p>
      </div>
    )
  }

  return (
    <div className="max-w-[640px] mx-auto space-y-4" data-testid="manage-token-page">
      <div>
        <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Manage token</h2>
        <p className="text-xs break-all" style={{ color: 'var(--ink-dim)' }}>
          {addr}
        </p>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Not a faucet. Wasm upgrades are CMM-only.
        </p>
      </div>

      {unverifiedAdmin && (
        <div className="shell-panel-strong" data-testid="unverified-admin-banner">
          Unverified admin — wasm admin is not CMM.
        </div>
      )}

      {!isManager && (
        <p className="text-sm" data-testid="manage-readonly">
          Read-only. Connect the manager wallet to submit.
        </p>
      )}

      <section className="shell-panel-strong space-y-3">
        <h3 className="font-heading uppercase text-sm">Taxes</h3>
        <div className="grid grid-cols-2 gap-3">
          <label>
            <span className="label-glass">Buy bps</span>
            <input
              className="input-glass w-full"
              disabled={!isManager}
              value={buyBps}
              placeholder={cfg ? String(cfg.buy_bps) : ''}
              onChange={(e) => setBuyBps(e.target.value)}
              data-testid="manage-buy-bps"
            />
          </label>
          <label>
            <span className="label-glass">Sell bps</span>
            <input
              className="input-glass w-full"
              disabled={!isManager}
              value={sellBps}
              placeholder={cfg ? String(cfg.sell_bps) : ''}
              onChange={(e) => setSellBps(e.target.value)}
              data-testid="manage-sell-bps"
            />
          </label>
        </div>
        <label>
          <span className="label-glass">Treasury</span>
          <input
            className="input-glass w-full"
            disabled={!isManager}
            value={treasury}
            placeholder={cfg?.treasury}
            onChange={(e) => setTreasury(e.target.value)}
          />
        </label>
        <label>
          <span className="label-glass">Wallet-to-wallet tax (bps)</span>
          <input
            className="input-glass w-full"
            disabled={!isManager || !feats?.transfer_tax}
            value={transferBps}
            placeholder={feats?.transfer_tax ? String(cfg?.transfer_bps ?? '') : 'Locked — unlock SKU'}
            onChange={(e) => setTransferBps(e.target.value)}
            data-testid="manage-transfer-bps"
          />
        </label>
        {feats?.exemption_directory && (
          <label>
            <span className="label-glass">Add exemption</span>
            <input
              className="input-glass w-full"
              disabled={!isManager}
              value={exemptAdd}
              onChange={(e) => setExemptAdd(e.target.value)}
            />
            <span className="mt-1 block text-xs text-[var(--text-muted)]">
              Skips buy, sell, and transfer tax. Launch guards still apply.
            </span>
          </label>
        )}
        {feats?.mint_control && !cfg?.mint_revoked && (
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              disabled={!isManager}
              checked={revokeAck}
              onChange={(e) => setRevokeAck(e.target.checked)}
              data-testid="manage-revoke-mint"
            />
            Revoke mint (one-way)
          </label>
        )}

        <p className="text-sm" data-testid="manage-save-copy">
          Save settings batch → 50 UST1
        </p>
        {saveInvoice && isManager ? (
          <PayWithAnyToken invoice={saveInvoice} tokens={pickerTokens} cta="pay" onPaid={setSaveHash} />
        ) : (
          <button type="button" className="btn-primary w-full" disabled data-testid="manage-save-disabled">
            Save settings
          </button>
        )}
        {saveHash && <TxResultAlert type="success" message="Settings saved." txHash={saveHash} />}
      </section>

      <section className="shell-panel-strong space-y-3">
        <h3 className="font-heading uppercase text-sm">Enable feature</h3>
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
          50 UST1 per SKU. Minting cannot be purchased here.
        </p>
        <select
          className="input-glass w-full"
          disabled={!isManager}
          value={unlockSku}
          onChange={(e) => setUnlockSku(e.target.value as CommunityTaxSkuId | '')}
          data-testid="manage-unlock-sku"
        >
          <option value="">Select a feature</option>
          {COMMUNITY_TAX_SKUS.filter((s) => isUnlockableAfterCreate(s.id) && !featureOn(feats, s.id)).map((s) => (
            <option key={s.id} value={s.id}>
              {s.label}
            </option>
          ))}
        </select>
        {unlockInvoice && isManager ? (
          <PayWithAnyToken invoice={unlockInvoice} tokens={pickerTokens} cta="enable" />
        ) : null}
      </section>

      {feats?.mint_control && !cfg?.mint_revoked && (
        <section className="shell-panel-strong space-y-3">
          <h3 className="font-heading uppercase text-sm">Mint</h3>
          <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
            Issuing supply is not a settings invoice.
          </p>
          <input
            className="input-glass w-full"
            disabled={!isManager}
            placeholder="Recipient"
            value={mintTo}
            onChange={(e) => setMintTo(e.target.value)}
          />
          <input
            className="input-glass w-full"
            disabled={!isManager}
            placeholder="Amount (raw)"
            value={mintAmount}
            onChange={(e) => setMintAmount(e.target.value)}
          />
          <button
            type="button"
            className="btn-primary w-full"
            disabled={!isManager || mintMut.isPending || !mintTo || !mintAmount}
            onClick={() => mintMut.mutate()}
          >
            Mint
          </button>
        </section>
      )}

      {cfg?.autolp && (
        <section className="shell-panel-strong space-y-3">
          <h3 className="font-heading uppercase text-sm">Auto liquidity</h3>
          <button
            type="button"
            className="btn-primary w-full"
            disabled={!address || skimMut.isPending}
            onClick={() => skimMut.mutate()}
            data-testid="manage-skim"
          >
            Skim to LP
          </button>
        </section>
      )}

      <p className="text-sm">
        <Link className="underline" to="/create">
          Create Pair
        </Link>{' '}
        ·{' '}
        <Link className="underline" to="/tokens">
          My tokens
        </Link>
      </p>
    </div>
  )
}

function featureOn(feats: CommunityTaxFeaturesResponse | undefined, id: CommunityTaxSkuId): boolean {
  return Boolean(feats?.[id])
}
