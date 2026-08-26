import { useMemo, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { TokenSearchSelect } from '@/components/trade/TokenSearchSelect'
import { PoolPreSubmitSummary } from '@/components/pool/PoolPreSubmitSummary'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { TxResultAlert } from '@/components/ui'
import { AddressRow } from '@/components/ui/AddressRow'
import { useWalletStore } from '@/hooks/useWallet'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { usePairPaused } from '@/hooks/usePairPaused'
import { usePairCodeIdFreeze } from '@/hooks/usePairCodeIdFreeze'
import { CODE_ID_FROZEN_CTA } from '@/utils/assetCodeIdFreeze'
import { useFeeDiscountRegistryStatus } from '@/hooks/useFeeDiscountRegistryStatus'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { getPool } from '@/services/terraclassic/pair'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { executeTerraContractMulti } from '@/services/terraclassic/transactions'
import { queryWrapMapperConfig, wrapMapperFeeBps, wrapTreasuryMatchesEnv } from '@/services/terraclassic/wrapMapper'
import { WRAP_MAPPER_CONTRACT_ADDRESS, isNativeWrapEnabled } from '@/utils/constants'
import { useDexStore } from '@/stores/dex'
import { assetInfoLabel, getNativeEquivalent, tokenAssetInfo, type PairInfo } from '@/types'
import { getDecimals, toRawAmount, formatTokenAmount, fromRawAmount } from '@/utils/formatAmount'
import {
  PAIR_LP_CW20_DECIMALS,
  conservativeZapOutExecution,
  nativeAfterZapUnwrap,
  resolveZapOutputKind,
  zapOutSplit,
  effectivePoolFeeBps,
} from '@/utils/oneSidedLiquidity'
import { fetchNativeTransferTaxParams } from '@/utils/nativeTransferTax'
import { buildZapOutMessages } from '@/utils/oneSidedLiquidityTx'
import { withdrawMinAssetAmounts } from '@/utils/rawAmountMath'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import { sounds } from '@/lib/sounds'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { ONE_SIDED_WITHDRAW_TITLE } from '@/utils/oneSidedLiquidityCopy'
import { ONE_SIDED_EMPTY_POOL_ERROR } from '@/utils/oneSidedLiquidityQuote'

export function OneSidedWithdrawCard({ pair }: { pair: PairInfo }) {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const queryClient = useQueryClient()
  const slippageTolerance = useDexStore((s) => s.slippageTolerance)

  const [asToken, setAsToken] = useState('')
  const [amount, setAmount] = useState('')
  const { discountBps } = useFeeDiscountRegistryStatus(pair.contract_addr)

  const lpBalanceQuery = useQuery({
    queryKey: ['lpBalance', address, pair.liquidity_token],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTokenBalance(address, tokenAssetInfo(pair.liquidity_token))
    },
    enabled: !!address,
    refetchInterval: 15_000,
  })
  const lpBalanceRaw = lpBalanceQuery.data ?? '0'
  const hasLp = !!address && lpBalanceRaw !== '0'

  const legs = useMemo(
    () => [assetInfoLabel(pair.asset_infos[0]), assetInfoLabel(pair.asset_infos[1])] as [string, string],
    [pair]
  )
  const asTokens = useMemo(() => {
    const ids = [...legs]
    if (isNativeWrapEnabled()) {
      for (const leg of legs) {
        const native = getNativeEquivalent(leg)
        if (native) ids.push(native)
      }
    }
    return ids
  }, [legs])

  const tradingBlacklist = useTradingBlacklist({
    wallet: address,
    token0: legs[0],
    token1: legs[1],
    pairAddress: pair.contract_addr,
    enabled: !!address,
  })
  const pairPaused = usePairPaused({ pairAddress: pair.contract_addr })
  const pairCodeIdFreeze = usePairCodeIdFreeze({ pairAddress: pair.contract_addr })

  const rawLp = amount ? toRawAmount(amount, PAIR_LP_CW20_DECIMALS) : '0'
  const debouncedRaw = useDebouncedValue(rawLp, SIM_QUOTE_DEBOUNCE_MS)

  const wrapMapperConfigQuery = useQuery({
    queryKey: ['wrapMapperConfig'],
    queryFn: queryWrapMapperConfig,
    enabled: !!WRAP_MAPPER_CONTRACT_ADDRESS,
    staleTime: 30_000,
  })
  const unwrapFeeBps = wrapMapperFeeBps(wrapMapperConfigQuery.data ?? null, 'unwrap')
  const wrapBlocked =
    (asToken === 'uluna' || asToken === 'uusd') &&
    !!WRAP_MAPPER_CONTRACT_ADDRESS &&
    (wrapMapperConfigQuery.data == null || !wrapTreasuryMatchesEnv(wrapMapperConfigQuery.data))

  const poolQuery = useQuery({
    queryKey: ['pool', pair.contract_addr],
    queryFn: () => getPool(pair.contract_addr),
    staleTime: 15_000,
  })
  const feeQuery = useQuery({
    queryKey: ['feeConfig', pair.contract_addr],
    queryFn: () => getPairFeeConfig(pair.contract_addr),
    staleTime: 60_000,
  })

  const kind = asToken ? resolveZapOutputKind(asToken, legs[0], legs[1]) : null
  const split =
    poolQuery.data && feeQuery.data && kind?.kind === 'pair_leg' && debouncedRaw !== '0'
      ? zapOutSplit({
          lpRaw: BigInt(debouncedRaw),
          totalShare: BigInt(poolQuery.data.total_share),
          reserveA: BigInt(poolQuery.data.assets[0].amount),
          reserveB: BigInt(poolQuery.data.assets[1].amount),
          wantSide: kind.side,
          feeBps: effectivePoolFeeBps(feeQuery.data.fee_bps, discountBps),
        })
      : null

  const isQuoteStale = rawLp !== debouncedRaw
  const minAssets =
    poolQuery.data && debouncedRaw !== '0'
      ? withdrawMinAssetAmounts(
          debouncedRaw,
          poolQuery.data.total_share,
          poolQuery.data.assets[0].amount,
          poolQuery.data.assets[1].amount,
          slippageTolerance
        )
      : null

  const unwrapNative = kind?.kind === 'pair_leg' && !!kind.wrapFromNative
  const exec =
    split?.status === 'ok' && minAssets && kind?.kind === 'pair_leg'
      ? conservativeZapOutExecution({
          split,
          wantSide: kind.side,
          minAssets,
          slippagePercent: slippageTolerance,
        })
      : null
  const unwrapAmount = exec && unwrapNative ? exec.unwrapAmount : null
  const nativeTaxQuery = useQuery({
    queryKey: ['nativeTransferTax', kind?.kind === 'pair_leg' ? kind.wrapFromNative : null],
    queryFn: () => fetchNativeTransferTaxParams(kind?.kind === 'pair_leg' ? (kind.wrapFromNative ?? 'uluna') : 'uluna'),
    enabled: unwrapNative,
    staleTime: 60_000,
  })

  const insufficient = !!address && rawLp !== '0' && BigInt(rawLp) > BigInt(lpBalanceRaw)

  const withdrawMutation = useTerraBroadcastMutation({
    toastSuccess: 'Liquidity withdrawn.',
    mutationFn: async () => {
      if (!address || !split || split.status !== 'ok' || !minAssets || !exec || !kind || kind.kind !== 'pair_leg') {
        throw new Error('Quote unavailable')
      }
      const wantedCw20 = kind.side === 'a' ? legs[0] : legs[1]
      const otherCw20 = kind.side === 'a' ? legs[1] : legs[0]
      const msgs = buildZapOutMessages({
        pairAddress: pair.contract_addr,
        lpToken: pair.liquidity_token,
        lpAmount: rawLp,
        minAssets,
        tokenAsk: otherCw20,
        swapAmount: exec.swapAmount,
        swapMinReturn: exec.swapMinReturn,
        slippagePercent: slippageTolerance,
        unwrap: unwrapAmount ? { cw20: wantedCw20, amount: unwrapAmount } : null,
      })
      return executeTerraContractMulti(address, msgs)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setAmount('')
      void queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      void queryClient.invalidateQueries({ queryKey: ['pool'] })
      void queryClient.invalidateQueries({ queryKey: ['lpBalance'] })
      void queryClient.invalidateQueries({ queryKey: ['portfolio-lp-balances'] })
    },
    onError: () => sounds.playError(),
  })

  const emptyPool =
    poolQuery.data != null && (poolQuery.data.assets[0].amount === '0' || poolQuery.data.assets[1].amount === '0')
  const noRoute = !!asToken && kind?.kind === 'off_pair'
  const disableReason = emptyPool
    ? ONE_SIDED_EMPTY_POOL_ERROR
    : wrapBlocked
      ? wrapMapperConfigQuery.data == null
        ? 'Wrap config unavailable'
        : 'Wrap treasury misconfigured'
      : pairPaused.isPaused
        ? 'Pair is paused'
        : pairCodeIdFreeze.isFrozen
          ? CODE_ID_FROZEN_CTA
          : tradingBlacklist.blocked
            ? 'Trading restricted'
            : insufficient
              ? 'Insufficient LP balance'
              : noRoute
                ? 'No route'
                : split?.status === 'unavailable'
                  ? 'Amount too small'
                  : split?.status === 'ok' && minAssets && !exec
                    ? 'Amount too small'
                    : isQuoteStale
                      ? 'Quote updating…'
                      : null

  const submitBlocked =
    withdrawMutation.isPending ||
    (!!address &&
      (!asToken ||
        !amount ||
        !split ||
        split.status !== 'ok' ||
        !minAssets ||
        !exec ||
        !!disableReason ||
        wrapBlocked ||
        pairPaused.isPaused ||
        pairCodeIdFreeze.isFrozen ||
        tradingBlacklist.blocked ||
        insufficient ||
        isQuoteStale))

  const nativePreview =
    unwrapAmount && unwrapFeeBps != null && nativeTaxQuery.data
      ? nativeAfterZapUnwrap(BigInt(unwrapAmount), unwrapFeeBps, nativeTaxQuery.data)
      : null
  const wantedDecimals =
    kind?.kind === 'pair_leg' && !kind.wrapFromNative
      ? getDecimals(tokenAssetInfo(kind.side === 'a' ? legs[0] : legs[1]))
      : asToken
        ? getDecimals(tokenAssetInfo(asToken))
        : 6

  return (
    <div className="card-glass space-y-3 animate-fade-in-up" data-testid="pool-one-sided-withdraw">
      {address && !lpBalanceQuery.isLoading && !hasLp && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="pool-one-sided-withdraw-empty-lp">
          No LP in this wallet for this pair.
        </p>
      )}
      <div>
        <label className="label-glass">LP</label>
        <p
          className="text-xs flex flex-wrap items-center gap-1"
          style={{ color: 'var(--ink-subtle)' }}
          data-testid="pool-one-sided-lp-pinned"
        >
          This pair’s LP{' '}
          <AddressRow
            address={pair.liquidity_token}
            startChars={8}
            endChars={6}
            copyAriaLabel="Copy LP token address"
            explorerAriaLabel="View LP token address on explorer"
            data-testid="pool-one-sided-lp-address-row"
          />
        </p>
      </div>
      <div>
        <label className="label-glass" htmlFor="pool-one-sided-withdraw-as">
          Withdraw as
        </label>
        <TokenSearchSelect
          id="pool-one-sided-withdraw-as"
          value={asToken}
          tokens={asTokens}
          onChange={setAsToken}
          aria-label="Withdraw as"
          disabled={asTokens.length === 0}
          loadingLabel="Select token"
        />
      </div>
      <div>
        <label className="label-glass" htmlFor="pool-one-sided-lp-amount">
          Amount
        </label>
        <input
          id="pool-one-sided-lp-amount"
          type="text"
          inputMode="decimal"
          value={amount}
          onChange={(e) => {
            const v = e.target.value
            if (v === '' || /^\d*\.?\d*$/.test(v)) setAmount(v)
          }}
          placeholder="0.00"
          className="input-glass"
          aria-label="Amount"
          data-testid="pool-one-sided-withdraw-amount"
        />
        {address && hasLp && (
          <AmountBalanceActions
            balanceQuery={lpBalanceQuery}
            decimals={PAIR_LP_CW20_DECIMALS}
            walletConnected={!!address}
            spendableRaw={BigInt(lpBalanceRaw)}
            onMax={() => setAmount(fromRawAmount(lpBalanceRaw, PAIR_LP_CW20_DECIMALS))}
            testIdMax="pool-one-sided-withdraw-max"
          />
        )}
      </div>
      {split?.status === 'ok' && !isQuoteStale && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="pool-one-sided-withdraw-quote">
          ~{formatTokenAmount(split.totalWantedCw20.toString(), wantedDecimals)}
          {nativePreview ? ` (~${formatTokenAmount(nativePreview.receive.toString(), 6)} native)` : ''}
        </p>
      )}
      {disableReason && (amount || emptyPool) && (
        <p className="text-xs font-semibold" style={{ color: 'var(--red, #ef4444)' }} role="alert">
          {disableReason}
        </p>
      )}
      {amount && (
        <PoolPreSubmitSummary
          actionLabel={ONE_SIDED_WITHDRAW_TITLE}
          pairLabel={`${legs[0]} / ${legs[1]}`}
          amountLines={[`${amount} LP`, asToken ? `as ${asToken}` : '']}
          data-testid="pool-one-sided-withdraw-pre-submit"
        />
      )}
      <button
        type="button"
        data-testid="pool-one-sided-withdraw-submit"
        disabled={!!address && submitBlocked}
        onClick={() => {
          sounds.playButtonPress()
          if (!address) openWalletModal()
          else withdrawMutation.mutate()
        }}
        className={`w-full py-2.5 font-semibold text-sm ${
          !!address && submitBlocked ? 'btn-disabled !w-full' : 'btn-primary !w-full'
        }`}
      >
        {!address
          ? 'Connect Wallet'
          : terraBroadcastPendingButtonLabel(
              withdrawMutation.phase,
              withdrawMutation.isPending,
              ONE_SIDED_WITHDRAW_TITLE,
              'Zapping…'
            )}
      </button>
      <TerraBroadcastPendingLink phase={withdrawMutation.phase} txHash={withdrawMutation.pendingTxHash} />
      {withdrawMutation.isError && (
        <TxResultAlert type="error" message={withdrawMutation.error?.message ?? 'Failed to withdraw'} />
      )}
      {withdrawMutation.isSuccess && (
        <TxResultAlert type="success" message="Liquidity withdrawn." txHash={withdrawMutation.data} />
      )}
    </div>
  )
}
