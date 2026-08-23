import { useEffect, useMemo, useState } from 'react'
import { Link, useSearchParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { PayWithAnyToken } from '@/components/payments/PayWithAnyToken'
import { TxResultAlert } from '@/components/ui'
import { useWalletStore } from '@/hooks/useWallet'
import { getTokens } from '@/services/indexer/client'
import { createFreeCommunityToken, queryLauncherConfig } from '@/services/terraclassic/communityTaxToken'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { COMMUNITY_TAX_PAIR_DIRECT_COPY } from '@/utils/taxPreviewMaxSpend'
import {
  COMMUNITY_TAX_SKUS,
  MAX_SINKS,
  skuInvoiceUst1RawString,
  type CommunityTaxSkuId,
  type SinkDraft,
  type SinkKindId,
} from '@/utils/communityTaxSku'
import { buildCreateTokenInvoice } from '@/utils/communityTaxInvoice'
import { buildValidatedCreateArgs } from '@/utils/communityTaxCreateForm'
import {
  autofillConnectedWallet,
  parseTokenDecimals,
  parseTokenName,
  parseTokenSymbol,
  walletOwnershipHelper,
} from '@/utils/communityTaxIdentity'
import {
  COMMUNITY_TOKEN_LAUNCHER,
  DOCS_GITLAB_BASE,
  isCommunityTaxEnabled,
  UST1_TOKEN_ADDRESS,
} from '@/utils/constants'
import { sounds } from '@/lib/sounds'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'

const EMPTY_SINK = (): SinkDraft => ({ kind: 'treasury', addr: '', percent: '' })

export default function CreateTokenPage() {
  const address = useWalletStore((s) => s.address)
  const [searchParams] = useSearchParams()
  void searchParams.get('manager')
  void searchParams.get('treasury')
  void searchParams.get('payee')

  const [name, setName] = useState('')
  const [symbol, setSymbol] = useState('')
  const [decimals, setDecimals] = useState('6')
  const [buyPct, setBuyPct] = useState('0')
  const [sellPct, setSellPct] = useState('0')
  const [transferPct, setTransferPct] = useState('0')
  const [treasury, setTreasury] = useState('')
  const [manager, setManager] = useState('')
  const [skus, setSkus] = useState<CommunityTaxSkuId[]>([])
  const [mintCap, setMintCap] = useState('')
  const [sinks, setSinks] = useState<SinkDraft[]>([EMPTY_SINK()])
  const [exemptList, setExemptList] = useState('')
  const [maxBuyPct, setMaxBuyPct] = useState('0')
  const [maxSellPct, setMaxSellPct] = useState('0')
  const [maxTransferPct, setMaxTransferPct] = useState('0')
  const [autolpThreshold, setAutolpThreshold] = useState('')
  const [autolpRecipient, setAutolpRecipient] = useState('')
  const [maxWallet, setMaxWallet] = useState('')
  const [cooldown, setCooldown] = useState('0')
  const [tradingEnabled, setTradingEnabled] = useState(false)
  const [ack, setAck] = useState(false)
  const [createdHash, setCreatedHash] = useState<string | null>(null)

  useEffect(() => {
    if (!address) return
    setTreasury((cur) => autofillConnectedWallet(cur, address))
    setManager((cur) => autofillConnectedWallet(cur, address))
    setAutolpRecipient((cur) => autofillConnectedWallet(cur, address))
  }, [address])

  const launcherQuery = useQuery({
    queryKey: ['communityTaxLauncherConfig'],
    queryFn: queryLauncherConfig,
    staleTime: 60_000,
    enabled: isCommunityTaxEnabled(),
  })
  const autolpCodeId = launcherQuery.data?.autolp_code_id ?? null

  const validated = useMemo(
    () =>
      buildValidatedCreateArgs({
        name,
        symbol,
        decimals,
        buyPercent: buyPct,
        sellPercent: sellPct,
        transferPercent: transferPct,
        treasury,
        manager,
        skus,
        mintCapHuman: mintCap,
        sinks,
        exemptList,
        maxBuyPercent: maxBuyPct,
        maxSellPercent: maxSellPct,
        maxTransferPercent: maxTransferPct,
        autolpThresholdHuman: autolpThreshold,
        autolpRecipient,
        maxWalletHuman: maxWallet,
        cooldownBlocks: cooldown,
        tradingEnabled,
        autolpCodeId,
      }),
    [
      name,
      symbol,
      decimals,
      buyPct,
      sellPct,
      transferPct,
      treasury,
      manager,
      skus,
      mintCap,
      sinks,
      exemptList,
      maxBuyPct,
      maxSellPct,
      maxTransferPct,
      autolpThreshold,
      autolpRecipient,
      maxWallet,
      cooldown,
      tradingEnabled,
      autolpCodeId,
    ]
  )

  const formOk = validated.ok && ack
  const hookArgs = formOk && validated.ok ? validated.args : null
  const fieldErrors = validated.ok ? {} : validated.errors

  const invoice =
    hookArgs && skus.length > 0 && UST1_TOKEN_ADDRESS && COMMUNITY_TOKEN_LAUNCHER
      ? buildCreateTokenInvoice({
          launcher: COMMUNITY_TOKEN_LAUNCHER,
          ust1: UST1_TOKEN_ADDRESS,
          args: hookArgs,
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

  const freeCreate = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address || !hookArgs) throw new Error('Wallet not connected')
      return createFreeCommunityToken(address, hookArgs)
    },
    onSuccess: (hash) => {
      sounds.playSuccess()
      setCreatedHash(hash)
    },
    onError: () => sounds.playError(),
  })

  const nameParsed = parseTokenName(name)
  const symbolParsed = parseTokenSymbol(symbol)
  const decParsed = parseTokenDecimals(decimals)

  if (!isCommunityTaxEnabled()) {
    return (
      <div className="max-w-[520px] mx-auto" data-testid="create-token-page">
        <div
          className="shell-panel-strong py-8 text-center"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="create-token-unavailable"
        >
          Create Token is not configured.
        </div>
      </div>
    )
  }

  const toggleSku = (id: CommunityTaxSkuId) => {
    setSkus((cur) => {
      const on = !cur.includes(id)
      const next = on ? [...cur, id] : cur.filter((s) => s !== id)
      if (!on) {
        if (id === 'transfer_tax') setTransferPct('0')
        if (id === 'split_router') setSinks([EMPTY_SINK()])
        if (id === 'exemption_directory') setExemptList('')
        if (id === 'variable_rates') {
          setMaxBuyPct('0')
          setMaxSellPct('0')
          setMaxTransferPct('0')
        }
        if (id === 'auto_v2_lp') {
          setAutolpThreshold('')
          setAutolpRecipient('')
        }
        if (id === 'launch_guards') {
          setMaxWallet('')
          setCooldown('0')
          setTradingEnabled(false)
        }
        if (id === 'mint_control') setMintCap('')
      }
      return next
    })
  }

  const fieldErr = (key: string) =>
    fieldErrors[key] ? (
      <p className="text-xs mt-1" style={{ color: 'var(--danger)' }} data-testid={`create-token-error-${key}`}>
        {fieldErrors[key]}
      </p>
    ) : null

  return (
    <div className="max-w-[520px] mx-auto" data-testid="create-token-page">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide font-heading">Create Token</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Community tax token. Tax is not the DEX swap fee. {COMMUNITY_TAX_PAIR_DIRECT_COPY} Wasm upgrades are CMM-only
          — you cannot migrate this contract.{' '}
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
          <span className="label-glass">Name</span>
          <input
            className="input-glass w-full"
            value={name}
            onChange={(e) => setName(e.target.value)}
            data-testid="create-token-name"
          />
          {name && !nameParsed.ok && (
            <p className="text-xs mt-1" style={{ color: 'var(--danger)' }}>
              {nameParsed.error}
            </p>
          )}
        </label>
        <label className="block">
          <span className="label-glass">Symbol</span>
          <input
            className="input-glass w-full"
            value={symbol}
            onChange={(e) => setSymbol(e.target.value)}
            data-testid="create-token-symbol"
          />
          {symbol && !symbolParsed.ok && (
            <p className="text-xs mt-1" style={{ color: 'var(--danger)' }}>
              {symbolParsed.error}
            </p>
          )}
        </label>
        <label className="block">
          <span className="label-glass">Decimals</span>
          <input
            className="input-glass w-full"
            value={decimals}
            onChange={(e) => setDecimals(e.target.value)}
            data-testid="create-token-decimals"
          />
          {!decParsed.ok && (
            <p className="text-xs mt-1" style={{ color: 'var(--danger)' }} data-testid="create-token-decimals-error">
              {decParsed.error}
            </p>
          )}
        </label>
        <div className="grid grid-cols-2 gap-3">
          <label>
            <span className="label-glass">Buy tax (%)</span>
            <input
              className="input-glass w-full"
              value={buyPct}
              onChange={(e) => setBuyPct(e.target.value)}
              data-testid="create-token-buy-pct"
            />
            {fieldErr('buy')}
          </label>
          <label>
            <span className="label-glass">Sell tax (%)</span>
            <input
              className="input-glass w-full"
              value={sellPct}
              onChange={(e) => setSellPct(e.target.value)}
              data-testid="create-token-sell-pct"
            />
            {fieldErr('sell')}
          </label>
        </div>
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="create-token-tax-scope">
          {COMMUNITY_TAX_PAIR_DIRECT_COPY} Up to 25.00% combined.
        </p>
        {fieldErrors.combined && (
          <p className="text-xs" style={{ color: 'var(--danger)' }}>
            {fieldErrors.combined}
          </p>
        )}
        <label className="block">
          <span className="label-glass">Treasury</span>
          <input
            className="input-glass w-full"
            value={treasury}
            onChange={(e) => setTreasury(e.target.value)}
            data-testid="create-token-treasury"
          />
          <p className="text-xs mt-1" style={{ color: 'var(--ink-dim)' }} data-testid="create-token-treasury-helper">
            {walletOwnershipHelper(treasury, address)}
          </p>
          {fieldErr('treasury')}
        </label>
        <label className="block">
          <span className="label-glass">Manager</span>
          <input
            className="input-glass w-full"
            value={manager}
            onChange={(e) => setManager(e.target.value)}
            data-testid="create-token-manager"
          />
          <p className="text-xs mt-1" style={{ color: 'var(--ink-dim)' }} data-testid="create-token-manager-helper">
            {walletOwnershipHelper(manager, address)}
          </p>
          {fieldErr('manager')}
        </label>

        <fieldset>
          <legend className="label-glass mb-2">Paid features (50 UST1 each)</legend>
          <p className="text-xs mb-2" style={{ color: 'var(--ink-dim)' }} data-testid="create-token-sku-total">
            SKU invoice: {skus.length === 0 ? '0' : Number(skuInvoiceUst1RawString(skus.length)) / 1_000_000} UST1
          </p>
          {COMMUNITY_TAX_SKUS.map((sku) => (
            <label key={sku.id} className="flex items-start gap-2 text-sm mb-2">
              <input
                type="checkbox"
                checked={skus.includes(sku.id)}
                onChange={() => toggleSku(sku.id)}
                data-testid={`create-token-sku-${sku.id}`}
              />
              <span>
                <strong>{sku.label}</strong>
                <span className="block text-xs" style={{ color: 'var(--ink-dim)' }}>
                  {sku.hint}
                </span>
              </span>
            </label>
          ))}
        </fieldset>

        {skus.includes('transfer_tax') && (
          <label className="block" data-testid="create-token-transfer-panel">
            <span className="label-glass">Wallet-to-wallet tax (%)</span>
            <input
              className="input-glass w-full"
              value={transferPct}
              onChange={(e) => setTransferPct(e.target.value)}
              data-testid="create-token-transfer-pct"
            />
            {fieldErr('transfer')}
          </label>
        )}

        {skus.includes('split_router') && (
          <div className="space-y-2" data-testid="create-token-sinks-panel">
            <span className="label-glass">Split treasury (shares must sum to 100.00%)</span>
            {sinks.map((row, i) => (
              <div key={i} className="grid grid-cols-3 gap-2">
                <select
                  className="input-glass"
                  value={row.kind}
                  onChange={(e) => {
                    const kind = e.target.value as SinkKindId
                    setSinks((cur) => cur.map((s, j) => (j === i ? { ...s, kind } : s)))
                  }}
                  data-testid={`create-token-sink-kind-${i}`}
                >
                  <option value="treasury">Treasury</option>
                  <option value="burn">Burn</option>
                  <option value="auto_lp">AutoLP</option>
                  <option value="wallet">Wallet</option>
                </select>
                <input
                  className="input-glass"
                  placeholder="%"
                  value={row.percent}
                  onChange={(e) =>
                    setSinks((cur) => cur.map((s, j) => (j === i ? { ...s, percent: e.target.value } : s)))
                  }
                  data-testid={`create-token-sink-pct-${i}`}
                />
                {row.kind === 'wallet' ? (
                  <input
                    className="input-glass"
                    placeholder="terra1…"
                    value={row.addr}
                    onChange={(e) =>
                      setSinks((cur) => cur.map((s, j) => (j === i ? { ...s, addr: e.target.value } : s)))
                    }
                    data-testid={`create-token-sink-addr-${i}`}
                  />
                ) : (
                  <span />
                )}
              </div>
            ))}
            <div className="flex gap-2">
              <button
                type="button"
                className="btn-secondary text-xs"
                disabled={sinks.length >= MAX_SINKS}
                onClick={() => setSinks((cur) => [...cur, EMPTY_SINK()])}
              >
                Add sink
              </button>
              {sinks.length > 1 && (
                <button
                  type="button"
                  className="btn-secondary text-xs"
                  onClick={() => setSinks((cur) => cur.slice(0, -1))}
                >
                  Remove
                </button>
              )}
            </div>
            {fieldErr('sinks')}
          </div>
        )}

        {skus.includes('auto_v2_lp') && (
          <div className="space-y-2" data-testid="create-token-autolp-panel">
            <label className="block">
              <span className="label-glass">AutoLP threshold (human)</span>
              <input
                className="input-glass w-full"
                value={autolpThreshold}
                onChange={(e) => setAutolpThreshold(e.target.value)}
                data-testid="create-token-autolp-threshold"
              />
            </label>
            <label className="block">
              <span className="label-glass">LP recipient</span>
              <input
                className="input-glass w-full"
                value={autolpRecipient}
                onChange={(e) => setAutolpRecipient(e.target.value)}
                data-testid="create-token-autolp-recipient"
              />
              <p className="text-xs mt-1" style={{ color: 'var(--ink-dim)' }}>
                {walletOwnershipHelper(autolpRecipient, address)}
              </p>
            </label>
            {fieldErr('autolp')}
            {fieldErr('autolpRecipient')}
          </div>
        )}

        {skus.includes('exemption_directory') && (
          <label className="block" data-testid="create-token-exempt-panel">
            <span className="label-glass">Extra exemptions (addresses)</span>
            <textarea
              className="input-glass w-full"
              value={exemptList}
              onChange={(e) => setExemptList(e.target.value)}
              data-testid="create-token-exempt-list"
            />
            {fieldErr('exempt')}
          </label>
        )}

        {skus.includes('variable_rates') && (
          <div className="space-y-2" data-testid="create-token-variable-panel">
            <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
              Max rates are immutable after create and cannot exceed 25.00% combined.
            </p>
            <label className="block">
              <span className="label-glass">Max buy (%)</span>
              <input
                className="input-glass w-full"
                value={maxBuyPct}
                onChange={(e) => setMaxBuyPct(e.target.value)}
                data-testid="create-token-max-buy-pct"
              />
              {fieldErr('maxBuy')}
            </label>
            <label className="block">
              <span className="label-glass">Max sell (%)</span>
              <input
                className="input-glass w-full"
                value={maxSellPct}
                onChange={(e) => setMaxSellPct(e.target.value)}
                data-testid="create-token-max-sell-pct"
              />
              {fieldErr('maxSell')}
            </label>
            <label className="block">
              <span className="label-glass">Max wallet-to-wallet (%)</span>
              <input
                className="input-glass w-full"
                value={maxTransferPct}
                onChange={(e) => setMaxTransferPct(e.target.value)}
                data-testid="create-token-max-transfer-pct"
              />
              {fieldErr('maxTransfer')}
            </label>
            {fieldErr('maxCombined')}
          </div>
        )}

        {skus.includes('launch_guards') && (
          <div className="space-y-2" data-testid="create-token-guards-panel">
            <label className="block">
              <span className="label-glass">Max wallet (human, optional)</span>
              <input
                className="input-glass w-full"
                value={maxWallet}
                onChange={(e) => setMaxWallet(e.target.value)}
                data-testid="create-token-max-wallet"
              />
              {fieldErr('maxWallet')}
            </label>
            <label className="block">
              <span className="label-glass">Cooldown blocks</span>
              <input
                className="input-glass w-full"
                value={cooldown}
                onChange={(e) => setCooldown(e.target.value)}
                data-testid="create-token-cooldown"
              />
              {fieldErr('cooldown')}
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={tradingEnabled}
                onChange={(e) => setTradingEnabled(e.target.checked)}
                data-testid="create-token-trading-enabled"
              />
              Trading enabled (default off)
            </label>
          </div>
        )}

        {skus.includes('mint_control') && (
          <label className="block">
            <span className="label-glass">Mint cap (optional, human)</span>
            <input
              className="input-glass w-full"
              value={mintCap}
              onChange={(e) => setMintCap(e.target.value)}
              data-testid="create-token-mint-cap"
            />
            {fieldErr('mintCap')}
          </label>
        )}

        <label className="flex items-start gap-2 text-sm">
          <input
            type="checkbox"
            checked={ack}
            onChange={(e) => setAck(e.target.checked)}
            data-testid="create-token-ack"
          />
          <span>I understand taxes and mint (if enabled) are controlled by the manager wallet.</span>
        </label>

        <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
          Review: wasm admin will be CMM governance, not you. This is not a faucet mint and not Create Pair.
        </p>

        {createdHash && <TxResultAlert type="success" message="Token created." txHash={createdHash} />}

        {!address ? (
          <p className="text-sm" style={{ color: 'var(--ink-dim)' }} data-testid="create-token-connect">
            Connect wallet to create
          </p>
        ) : skus.length === 0 ? (
          <button
            type="button"
            className="btn-primary w-full"
            disabled={!formOk || freeCreate.isPending}
            data-testid="create-token-free-cta"
            onClick={() => freeCreate.mutate()}
          >
            {terraBroadcastPendingButtonLabel(freeCreate.phase, freeCreate.isPending, 'Create Token', 'Creating…')}
          </button>
        ) : invoice && formOk ? (
          <div data-testid="create-token-pay">
            <p className="text-sm mb-2" data-testid="create-token-pay-copy">
              Create invoice → {Number(invoice.invoiceAmount) / 1_000_000} UST1
            </p>
            <PayWithAnyToken
              invoice={invoice}
              tokens={pickerTokens}
              cta="create"
              onPaid={(hash) => setCreatedHash(hash)}
            />
          </div>
        ) : (
          <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
            Finish the form to enable pay.
          </p>
        )}

        {freeCreate.isError && (
          <p className="text-sm" style={{ color: 'var(--danger)' }}>
            {humanizeUserFacingErrorFromUnknown(freeCreate.error)}
          </p>
        )}

        {createdHash && (
          <p className="text-sm">
            Next: copy the new token address from the explorer tx, then{' '}
            <Link className="underline" to="/create" data-testid="create-token-next-create-pair">
              Create Pair
            </Link>
            . Tokens are not auto-added to Swap.
          </p>
        )}

        <p className="text-sm">
          <Link className="underline" to="/tokens">
            My tokens
          </Link>
        </p>
      </div>
    </div>
  )
}
