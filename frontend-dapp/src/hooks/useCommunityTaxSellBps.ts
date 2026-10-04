import { queryContract } from '@/services/terraclassic/queries'
import { useQuery } from '@tanstack/react-query'
import {
  queryCommunityTaxConfig,
  queryCommunityTaxIsExempt,
  queryTaxPreview,
} from '@/services/terraclassic/communityTaxToken'
import { useWalletStore } from '@/hooks/useWallet'
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

async function assertNoSupportedLegacyTaxApi(token: string): Promise<void> {
  for (const name of ['tax_map', 'tax_info']) {
    try {
      await queryContract(token, { [name]: {} })
    } catch (error) {
      if (classifyCommunityTaxQueryError(error) === 'not_tax') continue
      throw error
    }
    throw new Error('Legacy token tax capability requires a supported route version')
  }
}

/**
 * LCD sell_bps for extra-debit Max + submit (#593 / #609 / #1267).
 * Catalog pin equality is **not** the sell detector — `GetConfig` on the pay contract is.
 * Trading discovery is independent of the Create Token feature configuration.
 * Previously observed tax remains protected after a failed config refetch.
 */
export function useCommunityTaxSellBps(tokenAddr: string | null | undefined) {
  const wallet = useWalletStore((s) => s.address)
  const enabled = !!tokenAddr && isValidTerraBech32Address(tokenAddr)
  const cfg = useQuery({
    queryKey: ['communityTaxSellBps', tokenAddr],
    queryFn: async () => {
      let config
      try {
        config = await queryCommunityTaxConfig(tokenAddr!)
      } catch (error) {
        if (classifyCommunityTaxQueryError(error) !== 'not_tax') throw error
        await assertNoSupportedLegacyTaxApi(tokenAddr!)
        throw error
      }
      if (config != null && typeof config === 'object' && 'sell_bps' in config) return config
      // A successful unrelated configuration schema is not tax attestation either.
      await assertNoSupportedLegacyTaxApi(tokenAddr!)
      throw new Error('Token tax configuration is malformed or unsupported')
    },
    enabled,
    staleTime: 30_000,
    retry: false,
  })
  const parsed = cfg.data != null ? parseCommunityTaxSellBps(cfg.data) : null
  let detection: CommunityTaxDetection = 'honest'
  if (enabled) {
    // React Query retains established configuration after a failed refetch.
    if (parsed?.kind === 'tax') {
      detection = 'tax'
    } else if (cfg.isLoading || parsed?.kind === 'unresolved') {
      detection = 'unresolved'
    } else if (cfg.isError) {
      detection = classifyCommunityTaxQueryError(cfg.error) === 'not_tax' ? 'honest' : 'unresolved'
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
  const managerExempt = isTax ? (exempt.isError ? null : (exempt.data?.manager ?? null)) : false
  return {
    sellBps: isTax ? effectiveExtraDebitSellBps(rawSellBps, managerExempt) : null,
    buyBps: isTax ? effectiveBuyTaxBps(cfg.data?.buy_bps ?? null, managerExempt) : null,
    isTaxToken: isTax,
    isLoading: cfg.isLoading || (isTax && exempt.isLoading),
    detection,
    extraDebitUnresolved: detection === 'unresolved' || (isTax && cfg.isError),
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
