import { useMemo, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { TokenSearchSelect } from '@/components/trade/TokenSearchSelect'
import { PoolPreSubmitSummary } from '@/components/pool/PoolPreSubmitSummary'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { TxResultAlert } from '@/components/ui'
import { useWalletStore } from '@/hooks/useWallet'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { useTokenBalance } from '@/hooks/useTokenBalance'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { usePairPaused } from '@/hooks/usePairPaused'
import { usePairCodeIdFreeze } from '@/hooks/usePairCodeIdFreeze'
import { CODE_ID_FROZEN_CTA } from '@/utils/assetCodeIdFreeze'
import { useFeeDiscountRegistryStatus } from '@/hooks/useFeeDiscountRegistryStatus'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { retailAddTokenCandidates, usePositiveWalletTokens } from '@/hooks/usePositiveWalletTokens'
import { getPool } from '@/services/terraclassic/pair'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { executeTerraContractMulti } from '@/services/terraclassic/transactions'
import { getAllTokens } from '@/services/terraclassic/router'
import { queryWrapMapperConfig, wrapMapperFeeBps, wrapTreasuryMatchesEnv } from '@/services/terraclassic/wrapMapper'
import { WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import { useDexStore } from '@/stores/dex'
import { assetInfoLabel, tokenAssetInfo, type PairInfo } from '@/types'
import { getDecimals, toRawAmount, formatTokenAmount } from '@/utils/formatAmount'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { pairInfoMenuLabel } from '@/utils/pairMenuOptions'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import { slippagePercentToDecimalString, buildZapInMessages } from '@/utils/oneSidedLiquidityTx'
import { PAIR_LP_CW20_DECIMALS } from '@/utils/oneSidedLiquidity'
import { oneSidedAddPreSignAmountLines, ONE_SIDED_ADD_TITLE } from '@/utils/oneSidedLiquidityCopy'
import { ONE_SIDED_EMPTY_POOL_ERROR, quoteOneSidedAdd } from '@/utils/oneSidedLiquidityQuote'
import { SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT } from '@/utils/swapRouteSlippage'
import { sounds } from '@/lib/sounds'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { evaluateProvideLiquidityCw20NativeGasGate } from '@/utils/provideLiquidityNativeGasBalanceGate'
import { estimateZapInUlunaFeesTotal } from '@/services/terraclassic/transactions'

const POOL_LP_RISK_DOC =
  'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/frontend.md#pool-lp-risk-disclosure'

export function OneSidedAddCard({ pair, factoryPairs }: { pair: PairInfo; factoryPairs: PairInfo[] }) {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const queryClient = useQueryClient()
  const slippageTolerance = useDexStore((s) => s.slippageTolerance)
  const expertMode = useDexStore((s) => s.expertMode)

  const [tokenId, setTokenId] = useState('')
  const [amount, setAmount] = useState('')
  const { discountBps } = useFeeDiscountRegistryStatus(pair.contract_addr)

  const factoryTokens = useMemo(() => getAllTokens(factoryPairs), [factoryPairs])
  const candidates = useMemo(() => retailAddTokenCandidates(factoryTokens), [factoryTokens])
  const holdings = usePositiveWalletTokens(address, candidates)

  const pairLabel = pairInfoMenuLabel(pair, { variant: 'full' })
  const legs = [assetInfoLabel(pair.asset_infos[0]), assetInfoLabel(pair.asset_infos[1])] as [string, string]

  const token0 = legs[0]
  const token1 = legs[1]
  const tradingBlacklist = useTradingBlacklist({
    wallet: address,
    token0,
    token1,
    pairAddress: pair.contract_addr,
    enabled: !!address,
  })
  const pairPaused = usePairPaused({ pairAddress: pair.contract_addr })
  const pairCodeIdFreeze = usePairCodeIdFreeze({ pairAddress: pair.contract_addr })

  const decimals = tokenId ? getDecimals(tokenAssetInfo(tokenId)) : 6
  const rawAmount = amount ? toRawAmount(amount, decimals) : '0'
  const debouncedRaw = useDebouncedValue(rawAmount, SIM_QUOTE_DEBOUNCE_MS)

  const balanceQuery = useTokenBalance(address, tokenId)
  const nativeUlunaQuery = useNativeUlunaBalance(address)

  const wrapMapperConfigQuery = useQuery({
    queryKey: ['wrapMapperConfig'],
    queryFn: queryWrapMapperConfig,
    enabled: !!WRAP_MAPPER_CONTRACT_ADDRESS,
    staleTime: 30_000,
  })
  const wrapFeeBps = wrapMapperFeeBps(wrapMapperConfigQuery.data ?? null, 'wrap')
  const wrapBlocked =
    !!tokenId &&
    (tokenId === 'uluna' || tokenId === 'uusd') &&
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

  const quoteQuery = useQuery({
    queryKey: [
      'one-sided-add-quote',
      tokenId,
      pair.contract_addr,
      debouncedRaw,
      slippageTolerance,
      discountBps,
      wrapFeeBps,
      address,
    ],
    enabled: !!tokenId && debouncedRaw !== '0' && !!poolQuery.data && feeQuery.data != null && !poolQuery.isFetching,
    queryFn: () =>
      quoteOneSidedAdd({
        tokenId,
        pair,
        pairLabel,
        payRaw: debouncedRaw,
        pool: poolQuery.data!,
        feeBps: feeQuery.data!.fee_bps,
        discountBps,
        wrapFeeBps,
        slippagePercent: slippageTolerance,
        maxSpreadStr: slippagePercentToDecimalString(slippageTolerance),
        trader: address ?? undefined,
      }),
  })

  const quote = quoteQuery.data
  const snapshot = quote?.status === 'ok' ? quote.snapshot : null
  const quoteDisable = quote?.status === 'unavailable' ? quote.disableReason : null
  const emptyPool =
    poolQuery.data != null && (poolQuery.data.assets[0].amount === '0' || poolQuery.data.assets[1].amount === '0')
  const isQuoteStale = rawAmount !== debouncedRaw || quoteQuery.isFetching || quoteQuery.isPlaceholderData
  const impactBlocked =
    snapshot?.priceImpactPercent != null &&
    snapshot.priceImpactPercent > SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT &&
    !expertMode

  const zapFees = useMemo(
    () =>
      estimateZapInUlunaFeesTotal({
        wrapDeposits: tokenId === 'uluna' || tokenId === 'uusd' ? 1 : 0,
        routeHops: snapshot?.routeIn ? snapshot.routeIn.operations.length : 0,
      }),
    [tokenId, snapshot]
  )
  const gasGate = useMemo(
    () => evaluateProvideLiquidityCw20NativeGasGate(amount, amount, decimals, decimals, nativeUlunaQuery, zapFees),
    [amount, decimals, nativeUlunaQuery, zapFees]
  )

  const maxResult = useMemo(
    () =>
      computeMaxSpendableHumanAmount({
        balanceRaw: balanceQuery.data ?? '0',
        decimals,
        assetIsNativeUluna: tokenId === 'uluna',
        context: 'zap_in',
        zapInHints: {
          wrapDeposits: tokenId === 'uluna' || tokenId === 'uusd' ? 1 : 0,
          routeHops: snapshot?.routeIn?.operations.length ?? 0,
        },
      }),
    [balanceQuery.data, decimals, tokenId, snapshot]
  )

  const insufficient =
    !!address &&
    !!amount &&
    balanceQuery.data != null &&
    rawAmount !== '0' &&
    BigInt(rawAmount) > BigInt(balanceQuery.data)

  const addMutation = useTerraBroadcastMutation({
    toastSuccess: 'Liquidity added.',
    mutationFn: async () => {
      if (!address || !snapshot) throw new Error('Quote unavailable')
      if (snapshot.payRaw !== rawAmount) throw new Error('Quote stale')
      const msgs = buildZapInMessages({
        pairAddress: snapshot.pairAddress,
        tokenOffer: snapshot.offerCw20,
        tokenAsk: snapshot.askCw20,
        wrapDenom: snapshot.wrapDenom,
        wrapGross: snapshot.wrapGross,
        routeIn: snapshot.routeIn,
        swapAmount: snapshot.swapAmount,
        swapMinReturn: snapshot.swapMinReturn,
        provideOffer: snapshot.provideOffer,
        provideAsk: snapshot.provideAsk,
        slippagePercent: snapshot.slippagePercent,
      })
      return executeTerraContractMulti(address, msgs)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setAmount('')
      void queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      void queryClient.invalidateQueries({ queryKey: ['pool'] })
      void queryClient.invalidateQueries({ queryKey: ['lpBalance'] })
    },
    onError: () => sounds.playError(),
  })

  const disableReason = emptyPool
    ? ONE_SIDED_EMPTY_POOL_ERROR
    : !tokenId
      ? null
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
                ? 'Insufficient balance'
                : !gasGate.canAddLiquidity
                  ? gasGate.userMessage
                  : quoteDisable
                    ? quoteDisable
                    : impactBlocked
                      ? `Enable Expert Mode`
                      : isQuoteStale
                        ? 'Quote updating…'
                        : null

  const submitBlocked =
    addMutation.isPending ||
    (!!address &&
      (!tokenId ||
        !amount ||
        !snapshot ||
        !!disableReason ||
        wrapBlocked ||
        pairPaused.isPaused ||
        pairCodeIdFreeze.isFrozen ||
        tradingBlacklist.blocked ||
        insufficient ||
        !gasGate.canAddLiquidity ||
        isQuoteStale ||
        impactBlocked))

  return (
    <div className="card-glass space-y-3 animate-fade-in-up" data-testid="pool-one-sided-add">
      <p
        className="text-[11px] sm:text-xs leading-relaxed"
        style={{ color: 'var(--ink-dim)' }}
        data-testid="pool-il-risk-notice"
        role="note"
      >
        <span className="font-semibold" style={{ color: 'var(--ink-subtle)' }}>
          Impermanent loss risk.
        </span>{' '}
        LP value can diverge from simply holding the underlying assets when pool prices move.{' '}
        <a href={POOL_LP_RISK_DOC} target="_blank" rel="noopener noreferrer" className="underline">
          Learn more
        </a>
      </p>
      {address && holdings.empty && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="pool-one-sided-add-empty-tokens">
          No tokens in this wallet.
        </p>
      )}
      <div>
        <label className="label-glass" htmlFor="pool-one-sided-token">
          Token
        </label>
        <TokenSearchSelect
          id="pool-one-sided-token"
          value={tokenId}
          tokens={holdings.tokenIds}
          onChange={setTokenId}
          aria-label="Token"
          disabled={!!address && holdings.empty}
          loadingLabel={holdings.loading ? 'Loading tokens…' : 'Select token'}
        />
      </div>
      <div>
        <label className="label-glass" htmlFor="pool-one-sided-amount">
          Amount
        </label>
        <input
          id="pool-one-sided-amount"
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
          data-testid="pool-one-sided-add-amount"
        />
        {address && tokenId && (
          <AmountBalanceActions
            balanceQuery={balanceQuery}
            decimals={decimals}
            walletConnected={!!address}
            spendableRaw={maxResult.spendableRaw}
            onMax={() => setAmount(maxResult.human)}
            testIdMax="pool-one-sided-add-max"
          />
        )}
      </div>
      {snapshot && !isQuoteStale && (
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="pool-one-sided-add-quote">
          {snapshot.wrapDenom ? 'Wrap + zap. ' : snapshot.routeIn ? 'Route + zap. ' : 'Zap. '}
          Est. LP ~{snapshot.estimatedLp ? formatTokenAmount(snapshot.estimatedLp, PAIR_LP_CW20_DECIMALS) : '—'}
        </p>
      )}
      {disableReason && (amount || emptyPool) && (
        <p className="text-xs font-semibold" style={{ color: 'var(--red, #ef4444)' }} role="alert">
          {disableReason}
        </p>
      )}
      {snapshot && amount && (
        <PoolPreSubmitSummary
          actionLabel={ONE_SIDED_ADD_TITLE}
          pairLabel={snapshot.pairLabel}
          amountLines={oneSidedAddPreSignAmountLines(
            amount,
            snapshot.swapMinReturn,
            getDecimals(tokenAssetInfo(snapshot.askCw20))
          )}
          data-testid="pool-one-sided-add-pre-submit"
        />
      )}
      <button
        type="button"
        data-testid="pool-one-sided-add-submit"
        disabled={!!address && submitBlocked}
        onClick={() => {
          sounds.playButtonPress()
          if (!address) openWalletModal()
          else addMutation.mutate()
        }}
        className={`w-full py-2.5 font-semibold text-sm ${
          !!address && submitBlocked ? 'btn-disabled !w-full' : 'btn-primary !w-full'
        }`}
      >
        {!address
          ? 'Connect Wallet'
          : terraBroadcastPendingButtonLabel(addMutation.phase, addMutation.isPending, ONE_SIDED_ADD_TITLE, 'Zapping…')}
      </button>
      <TerraBroadcastPendingLink phase={addMutation.phase} txHash={addMutation.pendingTxHash} />
      {addMutation.isError && <TxResultAlert type="error" message={addMutation.error?.message ?? 'Failed to add'} />}
      {addMutation.isSuccess && <TxResultAlert type="success" message="Liquidity added." txHash={addMutation.data} />}
    </div>
  )
}
