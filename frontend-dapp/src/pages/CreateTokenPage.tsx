import { useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { PayWithAnyToken } from '@/components/payments/PayWithAnyToken'
import { TxResultAlert } from '@/components/ui'
import { useWalletStore } from '@/hooks/useWallet'
import { getTokens } from '@/services/indexer/client'
import { createFreeCommunityToken } from '@/services/terraclassic/communityTaxToken'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import {
  COMMUNITY_TAX_SKUS,
  parseTaxBps,
  skuInvoiceUst1RawString,
  type CommunityTaxSkuId,
} from '@/utils/communityTaxSku'
import { buildCreateTokenInvoice, type CreateTokenHookArgs } from '@/utils/communityTaxInvoice'
import { COMMUNITY_TOKEN_LAUNCHER, isCommunityTaxEnabled, UST1_TOKEN_ADDRESS } from '@/utils/constants'
import { getTerraAddressInputError } from '@/utils/terraAddressValidation'
import { sounds } from '@/lib/sounds'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'

export default function CreateTokenPage() {
  const address = useWalletStore((s) => s.address)
  const [name, setName] = useState('')
  const [symbol, setSymbol] = useState('')
  const [decimals, setDecimals] = useState('6')
  const [buyBps, setBuyBps] = useState('0')
  const [sellBps, setSellBps] = useState('0')
  const [treasury, setTreasury] = useState('')
  const [manager, setManager] = useState('')
  const [skus, setSkus] = useState<CommunityTaxSkuId[]>([])
  const [mintCap, setMintCap] = useState('')
  const [ack, setAck] = useState(false)
  const [createdHash, setCreatedHash] = useState<string | null>(null)

  const managerAddr = manager.trim() || address || ''
  const treasuryAddr = treasury.trim() || managerAddr

  const buy = parseTaxBps(buyBps)
  const sell = parseTaxBps(sellBps)
  const treasuryErr = treasuryAddr ? getTerraAddressInputError(treasuryAddr) : 'Treasury required'
  const managerErr = managerAddr ? getTerraAddressInputError(managerAddr) : 'Manager required'
  const dec = Number(decimals)
  const decimalsOk = Number.isInteger(dec) && dec >= 0 && dec <= 18
  const symbolOk = /^[A-Za-z0-9]{1,12}$/.test(symbol.trim())
  const nameOk = name.trim().length >= 1 && name.trim().length <= 50

  const combinedBps = (buy.ok ? buy.bps : 0) + (sell.ok ? sell.bps : 0)
  const combinedOk = combinedBps <= 2500
  const formOk =
    nameOk && symbolOk && decimalsOk && buy.ok && sell.ok && combinedOk && !treasuryErr && !managerErr && ack

  const hookArgs: CreateTokenHookArgs | null = formOk
    ? {
        name: name.trim(),
        symbol: symbol.trim().toUpperCase(),
        decimals: dec,
        initialBalances: [],
        manager: managerAddr,
        treasury: treasuryAddr,
        buyBps: buy.ok ? buy.bps : 0,
        sellBps: sell.ok ? sell.bps : 0,
        features: skus,
        mint: skus.includes('mint_control') ? { minter: managerAddr, cap: mintCap.trim() || undefined } : undefined,
      }
    : null

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
    setSkus((cur) => (cur.includes(id) ? cur.filter((s) => s !== id) : [...cur, id]))
  }

  return (
    <div className="max-w-[520px] mx-auto" data-testid="create-token-page">
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-1 uppercase tracking-wide font-heading">Create Token</h2>
        <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
          Community tax token. Tax is not the DEX swap fee. Wasm upgrades are CMM-only — you cannot migrate this
          contract.{' '}
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
        </label>
        <label className="block">
          <span className="label-glass">Symbol</span>
          <input
            className="input-glass w-full"
            value={symbol}
            onChange={(e) => setSymbol(e.target.value)}
            data-testid="create-token-symbol"
          />
          {!symbolOk && symbol && (
            <p className="text-xs mt-1" style={{ color: 'var(--danger)' }}>
              1–12 letters or numbers
            </p>
          )}
        </label>
        <label className="block">
          <span className="label-glass">Decimals</span>
          <input className="input-glass w-full" value={decimals} onChange={(e) => setDecimals(e.target.value)} />
        </label>
        <div className="grid grid-cols-2 gap-3">
          <label>
            <span className="label-glass">Buy tax (bps)</span>
            <input
              className="input-glass w-full"
              value={buyBps}
              onChange={(e) => setBuyBps(e.target.value)}
              data-testid="create-token-buy-bps"
            />
            {!buy.ok && (
              <p className="text-xs mt-1" style={{ color: 'var(--danger)' }}>
                {buy.error}
              </p>
            )}
          </label>
          <label>
            <span className="label-glass">Sell tax (bps)</span>
            <input
              className="input-glass w-full"
              value={sellBps}
              onChange={(e) => setSellBps(e.target.value)}
              data-testid="create-token-sell-bps"
            />
            {!sell.ok && (
              <p className="text-xs mt-1" style={{ color: 'var(--danger)' }}>
                {sell.error}
              </p>
            )}
          </label>
        </div>
        {!combinedOk && (
          <p className="text-xs" style={{ color: 'var(--danger)' }}>
            Combined buy+sell tax cannot exceed 2500 bps.
          </p>
        )}
        <label className="block">
          <span className="label-glass">Treasury</span>
          <input
            className="input-glass w-full"
            value={treasury}
            onChange={(e) => setTreasury(e.target.value)}
            placeholder={address || 'terra1…'}
            data-testid="create-token-treasury"
          />
          {treasury && treasuryErr && (
            <p className="text-xs mt-1" style={{ color: 'var(--danger)' }}>
              {treasuryErr}
            </p>
          )}
        </label>
        <label className="block">
          <span className="label-glass">Manager</span>
          <input
            className="input-glass w-full"
            value={manager}
            onChange={(e) => setManager(e.target.value)}
            placeholder={address || 'Connected wallet'}
            data-testid="create-token-manager"
          />
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

        {skus.includes('mint_control') && (
          <label className="block">
            <span className="label-glass">Mint cap (optional, raw)</span>
            <input className="input-glass w-full" value={mintCap} onChange={(e) => setMintCap(e.target.value)} />
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

        {createdHash && <TxResultAlert variant="success" txHash={createdHash} />}

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
            {terraBroadcastPendingButtonLabel(freeCreate.isPending, 'Create Token')}
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
            <Link className="underline" to="/create">
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
