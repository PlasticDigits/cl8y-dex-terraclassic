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
  queryCommunityTaxTokenInfo,
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
  formatBpsAsPercent,
  isUnlockableAfterCreate,
  parseSharePercent,
  parseTaxPercent,
  type CommunityTaxSkuId,
} from '@/utils/communityTaxSku'
import { toRawAmount } from '@/utils/formatAmount'
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
  const [buyPct, setBuyPct] = useState('')
  const [sellPct, setSellPct] = useState('')
  const [treasury, setTreasury] = useState('')
  const [transferPct, setTransferPct] = useState('')
  const [exemptAdd, setExemptAdd] = useState('')
  const [sinkRows, setSinkRows] = useState<{ kind: string; addr: string; percent: string }[]>([])
  const [guardMaxWallet, setGuardMaxWallet] = useState('')
  const [guardCooldown, setGuardCooldown] = useState('')
  const [guardTrading, setGuardTrading] = useState(false)
  const [autolpPair, setAutolpPair] = useState('')
  const [autolpThreshold, setAutolpThreshold] = useState('')
  const [autolpRecipient, setAutolpRecipient] = useState('')
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
  const infoTokQuery = useQuery({
    queryKey: ['communityTaxTokenInfo', addr],
    queryFn: () => queryCommunityTaxTokenInfo(addr),
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

  const buy = parseTaxPercent(buyPct || (cfg ? formatBpsAsPercent(cfg.buy_bps) : '0'))
  const sell = parseTaxPercent(sellPct || (cfg ? formatBpsAsPercent(cfg.sell_bps) : '0'))
  const transfer = parseTaxPercent(transferPct || (cfg ? formatBpsAsPercent(cfg.transfer_bps) : '0'))
  const treasuryDraft = treasury.trim() || cfg?.treasury || ''
  const treasuryErr = treasuryDraft ? getTerraAddressInputError(treasuryDraft) : null
  const decimals = infoTokQuery.data?.decimals ?? 6

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
  if (feats?.split_router && sinkRows.length > 0) {
    const parsed = sinkRows.map((r) => ({
      kind: r.kind,
      addr: r.addr.trim() || undefined,
      share: parseSharePercent(r.percent),
    }))
    if (parsed.every((p) => p.share.ok)) {
      const sinks = parsed.map((p) => ({
        kind: p.kind,
        addr: p.addr,
        bps: p.share.ok ? p.share.bps : 0,
      }))
      const sum = sinks.reduce((a, s) => a + s.bps, 0)
      if (sum === 10_000) settings.sinks = sinks
    }
  }
  if (
    feats?.launch_guards &&
    (guardMaxWallet.trim() || guardCooldown.trim() || guardTrading !== Boolean(cfg?.launch_guards?.trading_enabled))
  ) {
    const cooldown = guardCooldown.trim() ? Number(guardCooldown.trim()) : (cfg?.launch_guards?.cooldown_blocks ?? 0)
    if (Number.isInteger(cooldown) && cooldown >= 0) {
      settings.launch_guards = {
        max_wallet: guardMaxWallet.trim()
          ? toRawAmount(guardMaxWallet.trim(), decimals)
          : (cfg?.launch_guards?.max_wallet ?? undefined),
        cooldown_blocks: cooldown,
        trading_enabled: guardTrading,
      }
    }
  }
  if (feats?.auto_v2_lp && cfg?.autolp && (autolpPair.trim() || autolpThreshold.trim() || autolpRecipient.trim())) {
    const recip = autolpRecipient.trim() || address || ''
    if (recip && !getTerraAddressInputError(recip)) {
      settings.autolp = {
        pair: autolpPair.trim() || undefined,
        threshold: autolpThreshold.trim() ? toRawAmount(autolpThreshold.trim(), decimals) : '1',
        lp_recipient: recip,
      }
    }
  }

  const transferDirtyLocked = Boolean(transferPct.trim()) && !feats?.transfer_tax
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
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
          Up to 25.00% combined.
        </p>
        <div className="grid grid-cols-2 gap-3">
          <label>
            <span className="label-glass">Buy tax (%)</span>
            <input
              className="input-glass w-full"
              disabled={!isManager}
              value={buyPct}
              placeholder={cfg ? formatBpsAsPercent(cfg.buy_bps) : ''}
              onChange={(e) => setBuyPct(e.target.value)}
              data-testid="manage-buy-pct"
            />
          </label>
          <label>
            <span className="label-glass">Sell tax (%)</span>
            <input
              className="input-glass w-full"
              disabled={!isManager}
              value={sellPct}
              placeholder={cfg ? formatBpsAsPercent(cfg.sell_bps) : ''}
              onChange={(e) => setSellPct(e.target.value)}
              data-testid="manage-sell-pct"
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
          <span className="label-glass">Wallet-to-wallet tax (%)</span>
          <input
            className="input-glass w-full"
            disabled={!isManager || !feats?.transfer_tax}
            value={transferPct}
            placeholder={feats?.transfer_tax ? formatBpsAsPercent(cfg?.transfer_bps ?? 0) : 'Locked — unlock SKU'}
            onChange={(e) => setTransferPct(e.target.value)}
            data-testid="manage-transfer-pct"
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
          </label>
        )}
        {feats?.split_router && (
          <div className="space-y-2" data-testid="manage-sinks-editor">
            <span className="label-glass">Split treasury (sum 100.00%)</span>
            {sinkRows.map((row, i) => (
              <div key={i} className="grid grid-cols-3 gap-2">
                <select
                  className="input-glass"
                  disabled={!isManager}
                  value={row.kind}
                  onChange={(e) =>
                    setSinkRows((cur) => cur.map((s, j) => (j === i ? { ...s, kind: e.target.value } : s)))
                  }
                >
                  <option value="treasury">Treasury</option>
                  <option value="burn">Burn</option>
                  <option value="auto_lp">AutoLP</option>
                  <option value="wallet">Wallet</option>
                </select>
                <input
                  className="input-glass"
                  disabled={!isManager}
                  placeholder="%"
                  value={row.percent}
                  onChange={(e) =>
                    setSinkRows((cur) => cur.map((s, j) => (j === i ? { ...s, percent: e.target.value } : s)))
                  }
                />
                <input
                  className="input-glass"
                  disabled={!isManager || row.kind !== 'wallet'}
                  placeholder="terra1…"
                  value={row.addr}
                  onChange={(e) =>
                    setSinkRows((cur) => cur.map((s, j) => (j === i ? { ...s, addr: e.target.value } : s)))
                  }
                />
              </div>
            ))}
            <button
              type="button"
              className="btn-secondary text-xs"
              disabled={!isManager || sinkRows.length >= 4}
              onClick={() => setSinkRows((cur) => [...cur, { kind: 'treasury', addr: '', percent: '' }])}
            >
              Add sink
            </button>
          </div>
        )}
        {feats?.launch_guards && (
          <div className="space-y-2" data-testid="manage-guards-editor">
            <span className="label-glass">Launch guards</span>
            <input
              className="input-glass w-full"
              disabled={!isManager}
              placeholder="Max wallet (human)"
              value={guardMaxWallet}
              onChange={(e) => setGuardMaxWallet(e.target.value)}
            />
            <input
              className="input-glass w-full"
              disabled={!isManager}
              placeholder={cfg?.launch_guards ? String(cfg.launch_guards.cooldown_blocks) : 'Cooldown blocks'}
              value={guardCooldown}
              onChange={(e) => setGuardCooldown(e.target.value)}
            />
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                disabled={!isManager}
                checked={guardTrading}
                onChange={(e) => setGuardTrading(e.target.checked)}
              />
              Trading enabled
            </label>
          </div>
        )}
        {feats?.auto_v2_lp && (
          <div className="space-y-2" data-testid="manage-autolp-editor">
            <span className="label-glass">Auto liquidity bind</span>
            {!cfg?.autolp && (
              <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
                Sister is not bound yet. Unlock Auto liquidity at create, or Enable Feature when the launcher has an
                AutoLP code id.
              </p>
            )}
            <input
              className="input-glass w-full"
              disabled={!isManager || !cfg?.autolp}
              placeholder="Listed pair"
              value={autolpPair}
              onChange={(e) => setAutolpPair(e.target.value)}
            />
            <input
              className="input-glass w-full"
              disabled={!isManager || !cfg?.autolp}
              placeholder="Threshold (human)"
              value={autolpThreshold}
              onChange={(e) => setAutolpThreshold(e.target.value)}
            />
            <input
              className="input-glass w-full"
              disabled={!isManager || !cfg?.autolp}
              placeholder="LP recipient"
              value={autolpRecipient}
              onChange={(e) => setAutolpRecipient(e.target.value)}
            />
          </div>
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
