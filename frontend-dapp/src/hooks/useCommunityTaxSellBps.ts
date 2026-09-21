import { useQuery } from '@tanstack/react-query'
import {
  queryCommunityTaxConfig,
  queryCommunityTaxIsExempt,
  queryTaxPreview,
} from '@/services/terraclassic/communityTaxToken'
import { useWalletStore } from '@/hooks/useWallet'
import { isCommunityTaxEnabled } from '@/utils/constants'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { effectiveBuyTaxBps } from '@/utils/communityTaxNetOut'
import {
  classifyCommunityTaxQueryError,
  effectiveExtraDebitSellBps,
  parseCommunityTaxSellBps,
  parseUintString,
  taxPreviewExecuteDebitRaw,
} from '@/utils/taxPreviewMaxSpend'
import type { CommunityTaxPreviewQuery } from '@/utils/communityTaxPreviewQuery'

export type CommunityTaxDetection = 'tax' | 'honest' | 'unresolved'

/**
 * LCD sell_bps for extra-debit Max + submit (#593 / #609 / #1267).
 * Catalog pin equality is **not** the sell detector — `GetConfig` on the pay contract is.
 * `isCommunityTaxEnabled()` only means Create Token env is present; it does not require
 * `code_id === VITE_COMMUNITY_TAX_CODE_ID` for this wallet's pay token.
 */
export function useCommunityTaxSellBps(tokenAddr: string | null | undefined) {
  const wallet = useWalletStore((s) => s.address)
  const enabled = isCommunityTaxEnabled() && !!tokenAddr && isValidTerraBech32Address(tokenAddr)
  const cfg = useQuery({
    queryKey: ['communityTaxSellBps', tokenAddr],
    queryFn: () => queryCommunityTaxConfig(tokenAddr!),
    enabled,
    staleTime: 30_000,
    retry: false,
  })
  const parsed = cfg.isSuccess ? parseCommunityTaxSellBps(cfg.data) : null
  let detection: CommunityTaxDetection = 'honest'
  if (enabled) {
    if (cfg.isLoading) {
      detection = 'unresolved'
    } else if (cfg.isError) {
      // Unknown-query and LCD transport: honest. TaxPreview fail-closed runs after
      // this instance is already classified tax (#1267 T13).
      detection = 'honest'
    } else if (parsed?.kind === 'tax') {
      detection = 'tax'
    } else if (parsed?.kind === 'unresolved') {
      detection = 'unresolved'
    }
  }
  const isTax = detection === 'tax'
  const rawSellBps = parsed?.kind === 'tax' ? parsed.sellBps : null
  const walletOk = !!wallet && isValidTerraBech32Address(wallet)
  const exempt = useQuery({
    queryKey: ['communityTaxIsExempt', tokenAddr, wallet],
    queryFn: () => queryCommunityTaxIsExempt(tokenAddr!, wallet!),
    enabled: enabled && isTax && walletOk,
    staleTime: 15_000,
    retry: false,
  })
  const managerExempt = isTax ? (exempt.data?.manager ?? null) : false
  return {
    sellBps: isTax ? effectiveExtraDebitSellBps(rawSellBps, managerExempt) : null,
    buyBps: isTax ? effectiveBuyTaxBps(cfg.data?.buy_bps ?? null, managerExempt) : null,
    isTaxToken: isTax,
    isLoading: cfg.isLoading || (isTax && exempt.isLoading),
    detection,
    extraDebitUnresolved: detection === 'unresolved',
  }
}

/**
 * Live execute-aligned `TaxPreview` debit (#1285): pair-direct `Send+Swap` send_msg;
 * router hop uses router→pair preview + `hop_trader_debit`.
 */
export function useCommunityTaxPreviewDebit(input: {
  token: string | null | undefined
  amount: string
  enabled: boolean
  previewQuery: CommunityTaxPreviewQuery | null
}) {
  const tokenOk = !!input.token && isValidTerraBech32Address(input.token)
  const amountOk = /^\d+$/.test(input.amount) && input.amount !== '0'
  const from = input.previewQuery?.from
  const to = input.previewQuery?.to
  const sendMsg = input.previewQuery?.sendMsg
  const fromOk = !!from && isValidTerraBech32Address(from)
  const toOk = !!to && isValidTerraBech32Address(to)
  const sendOk = typeof sendMsg === 'string' && sendMsg.length > 0
  const qEnabled = input.enabled && tokenOk && amountOk && fromOk && toOk && sendOk
  const q = useQuery({
    queryKey: ['communityTaxPreview', input.token, from, to, input.amount, sendMsg],
    queryFn: () =>
      queryTaxPreview({
        token: input.token!,
        from: from!,
        to: to!,
        amount: input.amount,
        sendMsg,
      }),
    enabled: qEnabled,
    staleTime: 5_000,
    retry: false,
  })
  let declaredRaw: bigint | null = null
  if (amountOk) {
    try {
      declaredRaw = BigInt(input.amount)
    } catch {
      declaredRaw = null
    }
  }
  const previewDebit = q.isSuccess ? parseUintString(q.data?.debit) : null
  const hopTraderDebit = q.isSuccess ? parseUintString(q.data?.hop_trader_debit) : null
  const debitRaw =
    q.isSuccess && declaredRaw != null ? taxPreviewExecuteDebitRaw({ declaredRaw, previewDebit, hopTraderDebit }) : null
  const previewQueryMissing = input.enabled && tokenOk && amountOk && !sendOk
  const previewErrUnresolved = q.isError && classifyCommunityTaxQueryError(q.error) === 'unresolved'
  const previewUnresolved =
    previewQueryMissing ||
    previewErrUnresolved ||
    (qEnabled && (q.isLoading || q.isError || (q.isSuccess && debitRaw == null)))
  return {
    debitRaw,
    previewUnresolved,
    isLoading: q.isLoading,
  }
}
