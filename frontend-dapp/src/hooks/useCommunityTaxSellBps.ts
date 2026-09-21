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
import { effectiveExtraDebitSellBps, parseCommunityTaxSellBps, parseUintString } from '@/utils/taxPreviewMaxSpend'

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
 * Live `TaxPreview.debit` for the execute path (`from=wallet`, `to=pair|router`, `amount=declared`).
 * Enabled only after sell detection is tax. Hostile/non-numeric debit stays unresolved.
 */
export function useCommunityTaxPreviewDebit(input: {
  token: string | null | undefined
  from: string | null | undefined
  to: string | null | undefined
  amount: string
  enabled: boolean
}) {
  const tokenOk = !!input.token && isValidTerraBech32Address(input.token)
  const fromOk = !!input.from && isValidTerraBech32Address(input.from)
  const toOk = !!input.to && isValidTerraBech32Address(input.to)
  const amountOk = /^\d+$/.test(input.amount) && input.amount !== '0'
  const qEnabled = input.enabled && tokenOk && fromOk && toOk && amountOk
  const q = useQuery({
    queryKey: ['communityTaxPreview', input.token, input.from, input.to, input.amount],
    queryFn: () =>
      queryTaxPreview({
        token: input.token!,
        from: input.from!,
        to: input.to!,
        amount: input.amount,
      }),
    enabled: qEnabled,
    staleTime: 5_000,
    retry: false,
  })
  const debitRaw = q.isSuccess ? parseUintString(q.data?.debit) : null
  const previewUnresolved = qEnabled && (q.isLoading || q.isError || (q.isSuccess && debitRaw == null))
  return {
    debitRaw,
    previewUnresolved,
    isLoading: q.isLoading,
  }
}
