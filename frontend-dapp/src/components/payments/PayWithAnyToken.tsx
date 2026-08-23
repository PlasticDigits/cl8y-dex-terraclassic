/**
 * Reusable pay-with-any-token card (GitLab #595).
 * Callers (#593 Create Token / manager Save, later #597) pass `Invoice` only.
 */

import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { TokenSearchSelect } from '@/components/trade/TokenSearchSelect'
import { SlippageProtectionPresets } from '@/components/common/SlippageProtectionPresets'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { TxResultAlert } from '@/components/ui'
import { useWalletStore } from '@/hooks/useWallet'
import { useTokenBalance } from '@/hooks/useTokenBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { executeTerraContractMulti } from '@/services/terraclassic/transactions'
import { estimateTerraClassicFeeForEntries } from '@/services/terraclassic/terraClassicFeeEstimate'
import { queryWrapMapperConfig, wrapMapperFeeBps } from '@/services/terraclassic/wrapMapper'
import { useDexStore } from '@/stores/dex'
import { isNativeDenom, tokenAssetInfo } from '@/types'
import { TRADE_SLIPPAGE_PRESET_CLASS } from '@/utils/tradeMoneyCta'
import { WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import { formatTokenAmount, fromRawAmount, getDecimals } from '@/utils/formatAmount'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import {
  buildPayInvoiceMsgs,
  defaultPayToken,
  payInvoicePickerTokens,
  quotePayInvoice,
  type Invoice,
} from '@/utils/payInvoice'
import { PAY_INVOICE_CALCULATING, payInvoiceCtaLabel, payInvoiceSummaryLine } from '@/utils/payInvoiceCopy'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { sounds } from '@/lib/sounds'

export type PayWithAnyTokenProps = {
  invoice: Invoice
  tokens: string[]
  invoiceSymbol?: string
  cta?: 'pay' | 'enable'
  onPaid?: (txHash: string) => void
}

function TokenSymbol({ tokenId }: { tokenId: string }) {
  const { symbol, displayLabel } = useTokenDisplayInfo(tokenAssetInfo(tokenId))
  return <>{symbol || displayLabel}</>
}

export function PayWithAnyToken({
  invoice,
  tokens,
  invoiceSymbol = 'UST1',
  cta = 'pay',
  onPaid,
}: PayWithAnyTokenProps) {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const slippageTolerance = useDexStore((s) => s.slippageTolerance)
  const setSlippageTolerance = useDexStore((s) => s.setSlippageTolerance)

  const pickerTokens = useMemo(() => payInvoicePickerTokens(tokens), [tokens])
  const [payToken, setPayToken] = useState(invoice.invoiceToken)

  const invoiceDec = getDecimals(tokenAssetInfo(invoice.invoiceToken))
  const payDec = getDecimals(tokenAssetInfo(payToken || invoice.invoiceToken))
  const invoiceHuman = fromRawAmount(invoice.invoiceAmount, invoiceDec)

  const wrapConfigQuery = useQuery({
    queryKey: ['wrapMapperConfig'],
    queryFn: queryWrapMapperConfig,
    enabled: !!WRAP_MAPPER_CONTRACT_ADDRESS,
    staleTime: 30_000,
  })
  const wrapFeeBps = wrapMapperFeeBps(wrapConfigQuery.data ?? null, 'wrap')

  const payBalanceQuery = useTokenBalance(address, payToken)
  const nativeUlunaQuery = useNativeUlunaBalance(address)
  const invoiceBalanceQuery = useTokenBalance(address, invoice.invoiceToken)

  useEffect(() => {
    const invoiceBal = invoiceBalanceQuery.data
    const next = defaultPayToken({
      invoiceToken: invoice.invoiceToken,
      invoiceAmount: invoice.invoiceAmount,
      pickerTokens,
      balances: invoiceBal ? { [invoice.invoiceToken]: invoiceBal } : {},
    })
    setPayToken((cur) => (cur ? cur : next))
  }, [invoice.invoiceAmount, invoice.invoiceToken, invoiceBalanceQuery.data, pickerTokens])

  const debouncedPay = useDebouncedValue(payToken, SIM_QUOTE_DEBOUNCE_MS)
  const quoteQuery = useQuery({
    queryKey: [
      'pay-invoice-quote',
      invoice.invoiceToken,
      invoice.invoiceAmount,
      invoice.payee,
      invoice.hookMsg,
      debouncedPay,
      slippageTolerance,
      address,
      payBalanceQuery.data,
      wrapFeeBps,
    ],
    queryFn: () =>
      quotePayInvoice({
        invoice,
        payToken: debouncedPay,
        slippagePercent: slippageTolerance,
        trader: address ?? undefined,
        payTokenBalance: isNativeDenom(debouncedPay) ? nativeUlunaQuery.data : payBalanceQuery.data,
        wrapFeeBps,
      }),
    enabled: !!debouncedPay && !!invoice.invoiceAmount,
    staleTime: 10_000,
  })

  const quote = quoteQuery.data
  const quoteOk = quote?.status === 'ok' ? quote : null
  const quoteDisable = quote?.status === 'unavailable' ? quote.disableReason : null

  const msgs = useMemo(() => {
    if (!address || !quoteOk) return []
    try {
      return buildPayInvoiceMsgs({
        invoice,
        quote: quoteOk,
        walletAddress: address,
        slippagePercent: slippageTolerance,
      })
    } catch {
      return []
    }
  }, [address, invoice, quoteOk, slippageTolerance])

  const feeEst = useMemo(() => (msgs.length ? estimateTerraClassicFeeForEntries(msgs) : null), [msgs])

  const payMutation = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address || msgs.length === 0) throw new Error('Quote updating…')
      return executeTerraContractMulti(address, msgs)
    },
    onSuccess: (txHash) => {
      sounds.playSuccess()
      onPaid?.(txHash)
    },
    onError: () => sounds.playError(),
  })

  const paySymbolQuery = useTokenDisplayInfo(tokenAssetInfo(payToken || invoice.invoiceToken))
  const summary = quoteOk
    ? payInvoiceSummaryLine({
        payHuman: formatTokenAmount(quoteOk.payRaw, payDec),
        paySymbol: paySymbolQuery.symbol || paySymbolQuery.displayLabel,
        invoiceHuman,
        invoiceSymbol,
        routed: quoteOk.kind !== 'direct',
      })
    : quoteQuery.isFetching
      ? PAY_INVOICE_CALCULATING
      : null

  const disableReason = !payToken ? null : quoteQuery.isFetching ? PAY_INVOICE_CALCULATING : quoteDisable

  const submitBlocked =
    payMutation.isPending || !!disableReason || !quoteOk || msgs.length === 0 || quoteQuery.isFetching

  return (
    <div className="shell-panel-strong space-y-3" data-testid="pay-with-any-token">
      <p className="text-sm font-semibold" data-testid="pay-invoice-amount">
        {invoiceHuman} {invoiceSymbol}
      </p>
      <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="pay-invoice-summary">
        {summary}
      </p>

      <label className="block text-xs font-medium" style={{ color: 'var(--ink-subtle)' }}>
        Pay with
        <div className="mt-1">
          <TokenSearchSelect value={payToken} tokens={pickerTokens} onChange={setPayToken} aria-label="Pay token" />
        </div>
      </label>

      {quoteOk && quoteOk.kind !== 'direct' && (
        <p className="text-xs" data-testid="pay-invoice-route">
          Route {quoteOk.routeLabel || <TokenSymbol tokenId={payToken} />}
        </p>
      )}

      <SlippageProtectionPresets
        selectedPercent={slippageTolerance}
        onSelect={setSlippageTolerance}
        chipClassName={TRADE_SLIPPAGE_PRESET_CLASS}
        groupTestId="pay-invoice-slippage-presets"
        presetTestIdPrefix="pay-invoice-slippage-preset-"
      />

      {feeEst && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="pay-invoice-network-fee">
          Network fee (est.) ~{fromRawAmount(feeEst.feeUluna.toString(), 6)} LUNC
        </p>
      )}

      {disableReason && disableReason !== PAY_INVOICE_CALCULATING && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="pay-invoice-disable">
          {disableReason}
        </p>
      )}

      {payMutation.isError && (
        <TxResultAlert type="error" message={humanizeUserFacingErrorFromUnknown(payMutation.error)} />
      )}

      {address ? (
        <button
          type="button"
          className="btn-primary w-full"
          data-testid="pay-invoice-cta"
          disabled={submitBlocked}
          onClick={() => {
            sounds.playButtonPress()
            payMutation.mutate()
          }}
        >
          {terraBroadcastPendingButtonLabel(
            payMutation.phase,
            payMutation.isPending,
            payInvoiceCtaLabel(cta),
            'Paying…'
          )}
        </button>
      ) : (
        <button
          type="button"
          className="btn-primary w-full"
          data-testid="pay-invoice-connect"
          onClick={openWalletModal}
        >
          Connect wallet
        </button>
      )}
      <TerraBroadcastPendingLink phase={payMutation.phase} txHash={payMutation.pendingTxHash} />
    </div>
  )
}
