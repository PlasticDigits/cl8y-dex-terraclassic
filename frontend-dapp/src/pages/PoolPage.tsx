import { useState, memo, useMemo, useId } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useWalletStore } from '@/hooks/useWallet'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { getPool, provideLiquidity, withdrawLiquidity } from '@/services/terraclassic/pair'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import {
  executeTerraContract,
  executeTerraContractMulti,
  estimateProvideLiquidityCw20SequenceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { netUlunaAfterTransferTaxAsync } from '@/utils/nativeTransferTax'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import {
  FEE_DISCOUNT_CONTRACT_ADDRESS,
  FACTORY_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
  WRAP_MAPPER_CONTRACT_ADDRESS,
} from '@/utils/constants'
import { FACTORY_PAIRS_MAX_FOR_POOL_LIST, getPairListBadges, type PairListBadges } from '@/utils/pairListBadges'
import type { AssetInfo, IndexerPairSort, PairInfo } from '@/types'
import { assetInfoLabel, tokenAssetInfo, getNativeEquivalent, indexerPairToPairInfo } from '@/types'
import { getPairs, getTokens } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, POOL_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import {
  Spinner,
  TokenDisplay,
  RetryError,
  Skeleton,
  FeeDisplay,
  TxResultAlert,
  MenuSelect,
  type MenuSelectOption,
} from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { sounds } from '@/lib/sounds'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'
import { pairInfoMenuLabel } from '@/utils/pairMenuOptions'
import { AddressRow } from '@/components/ui/AddressRow'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { formatTokenAmount, formatNum, getDecimals, toRawAmount, fromRawAmount } from '@/utils/formatAmount'
import { isLpBurnExceedsBalance, withdrawMinAssetAmounts } from '@/utils/rawAmountMath'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { estimateProvideLiquidityUserLp, isProportionalAddAmounts } from '@/utils/provideLiquidityEstimate'
import { evaluateProvideLiquidityCw20NativeGasGate } from '@/utils/provideLiquidityNativeGasBalanceGate'
import { isLcdConnectivityError, LCD_CONNECTIVITY_OUTAGE_MESSAGE } from '@/utils/lcdConnectivity'
import { getErrorMessage } from '@/utils/humanizeUserFacingError'

const POOL_SORT_OPTIONS: MenuSelectOption[] = [
  { value: 'symbol', label: 'Name (A–Z)' },
  { value: 'volume_24h', label: '24h volume' },
  { value: 'fee', label: 'Fee' },
  { value: 'created', label: 'Created' },
  { value: 'id', label: 'Pair ID' },
]

const ORDER_OPTIONS: MenuSelectOption[] = [
  { value: 'asc', label: 'Ascending' },
  { value: 'desc', label: 'Descending' },
]

const POOL_DATA_SOURCES_DOC =
  'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/frontend.md#liquidity-pools-list-indexer-vs-factory'

const PoolCard = memo(function PoolCard({
  pair,
  volumeQuote24h,
  listBadges,
}: {
  pair: PairInfo
  volumeQuote24h?: string
  listBadges: PairListBadges
}) {
  const address = useWalletStore((s) => s.address)
  const queryClient = useQueryClient()
  const [expanded, setExpanded] = useState<'add' | 'remove' | null>(null)
  const [amountA, setAmountA] = useState('')
  const [amountB, setAmountB] = useState('')
  const [lpAmount, setLpAmount] = useState('')
  const [withdrawSlippage, setWithdrawSlippage] = useState('1.0')
  const [useNativeA, setUseNativeA] = useState(false)
  const [useNativeB, setUseNativeB] = useState(false)
  const [receiveWrapped, setReceiveWrapped] = useState(true)
  const lpTokenAmountInputId = useId()

  const tokenA = assetInfoLabel(pair.asset_infos[0])
  const tokenB = assetInfoLabel(pair.asset_infos[1])
  const token0Addr = 'token' in pair.asset_infos[0] ? pair.asset_infos[0].token.contract_addr : null
  const token1Addr = 'token' in pair.asset_infos[1] ? pair.asset_infos[1].token.contract_addr : null
  const tradingBlacklist = useTradingBlacklist({
    wallet: address,
    token0: token0Addr,
    token1: token1Addr,
    pairAddress: pair.contract_addr,
    enabled: !!address,
  })
  const displayA = useTokenDisplayInfo(pair.asset_infos[0])
  const displayB = useTokenDisplayInfo(pair.asset_infos[1])

  const nativeEquivA = useMemo(() => getNativeEquivalent(tokenA), [tokenA])
  const nativeEquivB = useMemo(() => getNativeEquivalent(tokenB), [tokenB])
  const hasNativeOptionA = !!nativeEquivA
  const hasNativeOptionB = !!nativeEquivB

  const poolQuery = useQuery({
    queryKey: ['pool', pair.contract_addr],
    queryFn: () => getPool(pair.contract_addr),
    staleTime: 30_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', pair.contract_addr],
    queryFn: () => getPairFeeConfig(pair.contract_addr),
    staleTime: 60_000,
  })

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTraderDiscount(address)
    },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 15_000,
  })

  const lpBalanceQuery = useQuery({
    queryKey: ['lpBalance', address, pair.liquidity_token],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTokenBalance(address, tokenAssetInfo(pair.liquidity_token))
    },
    enabled: !!address && expanded === 'remove',
    refetchInterval: 15_000,
  })

  const balanceInfoA: AssetInfo = useMemo(
    () =>
      hasNativeOptionA && useNativeA && nativeEquivA
        ? { native_token: { denom: nativeEquivA } }
        : tokenAssetInfo(tokenA),
    [hasNativeOptionA, useNativeA, nativeEquivA, tokenA]
  )
  const balanceInfoB: AssetInfo = useMemo(
    () =>
      hasNativeOptionB && useNativeB && nativeEquivB
        ? { native_token: { denom: nativeEquivB } }
        : tokenAssetInfo(tokenB),
    [hasNativeOptionB, useNativeB, nativeEquivB, tokenB]
  )

  const balanceKeyA = useMemo(() => assetInfoLabel(balanceInfoA), [balanceInfoA])
  const balanceKeyB = useMemo(() => assetInfoLabel(balanceInfoB), [balanceInfoB])

  const balanceAQuery = useQuery({
    queryKey: ['tokenBalance', address, balanceKeyA],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTokenBalance(address, balanceInfoA)
    },
    enabled: !!address && expanded === 'add',
    refetchInterval: 15_000,
  })
  const balanceBQuery = useQuery({
    queryKey: ['tokenBalance', address, balanceKeyB],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTokenBalance(address, balanceInfoB)
    },
    enabled: !!address && expanded === 'add',
    refetchInterval: 15_000,
  })

  const nativeUlunaQuery = useNativeUlunaBalance(address)

  const LP_DECIMALS = 6
  const lpBalance = lpBalanceQuery.data ?? '0'
  const lpBalanceDisplay = lpBalance === '0' ? '0' : formatTokenAmount(lpBalance, LP_DECIMALS)
  const insufficientLp = isLpBurnExceedsBalance(lpAmount, LP_DECIMALS, lpBalance)

  const decimalsA = getDecimals(pair.asset_infos[0])
  const decimalsB = getDecimals(pair.asset_infos[1])

  const rawAddA = amountA ? toRawAmount(amountA, decimalsA) : '0'
  const rawAddB = amountB ? toRawAmount(amountB, decimalsB) : '0'

  const insufficientAddA =
    !!address &&
    !!amountA &&
    !balanceAQuery.isLoading &&
    !balanceAQuery.isError &&
    balanceAQuery.data != null &&
    rawAddA !== '0' &&
    BigInt(rawAddA) > BigInt(balanceAQuery.data)

  const insufficientAddB =
    !!address &&
    !!amountB &&
    !balanceBQuery.isLoading &&
    !balanceBQuery.isError &&
    balanceBQuery.data != null &&
    rawAddB !== '0' &&
    BigInt(rawAddB) > BigInt(balanceBQuery.data)

  const insufficientAdd = insufficientAddA || insufficientAddB

  const needsWrapA = hasNativeOptionA && useNativeA
  const needsWrapB = hasNativeOptionB && useNativeB

  const nativeWrapDepositCount = useMemo((): 1 | 2 => {
    if (needsWrapA && needsWrapB) return 2
    return 1
  }, [needsWrapA, needsWrapB])

  const maxResultA = useMemo(() => {
    if (!balanceAQuery.data) {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: balanceAQuery.data,
      decimals: decimalsA,
      assetIsNativeUluna: needsWrapA && nativeEquivA === 'uluna',
      context: needsWrapA ? 'provide_liquidity_native_side' : 'provide_liquidity_cw20',
      nativeWrapDepositCount: needsWrapA ? nativeWrapDepositCount : undefined,
    })
  }, [balanceAQuery.data, decimalsA, needsWrapA, nativeEquivA, nativeWrapDepositCount])

  const maxResultB = useMemo(() => {
    if (!balanceBQuery.data) {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: balanceBQuery.data,
      decimals: decimalsB,
      assetIsNativeUluna: needsWrapB && nativeEquivB === 'uluna',
      context: needsWrapB ? 'provide_liquidity_native_side' : 'provide_liquidity_cw20',
      nativeWrapDepositCount: needsWrapB ? nativeWrapDepositCount : undefined,
    })
  }, [balanceBQuery.data, decimalsB, needsWrapB, nativeEquivB, nativeWrapDepositCount])

  const provideCw20MinUlunaFees = useMemo(() => estimateProvideLiquidityCw20SequenceUlunaFeesTotal(), [])

  const provideLiquidityNativeGasGate = useMemo(() => {
    if (needsWrapA || needsWrapB) {
      return { canAddLiquidity: true, userMessage: null as string | null, tone: 'none' as const }
    }
    return evaluateProvideLiquidityCw20NativeGasGate(
      amountA,
      amountB,
      decimalsA,
      decimalsB,
      {
        data: nativeUlunaQuery.data,
        isLoading: nativeUlunaQuery.isLoading,
        isError: nativeUlunaQuery.isError,
      },
      provideCw20MinUlunaFees
    )
  }, [
    needsWrapA,
    needsWrapB,
    amountA,
    amountB,
    decimalsA,
    decimalsB,
    nativeUlunaQuery.data,
    nativeUlunaQuery.isLoading,
    nativeUlunaQuery.isError,
    provideCw20MinUlunaFees,
  ])

  const estimatedUserLp =
    poolQuery.data && amountA && amountB ? estimateProvideLiquidityUserLp(rawAddA, rawAddB, poolQuery.data) : null

  const ratioBalanced =
    poolQuery.data && amountA && amountB ? isProportionalAddAmounts(rawAddA, rawAddB, poolQuery.data) : null

  const addMutation = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      let rawA = toRawAmount(amountA, decimalsA)
      let rawB = toRawAmount(amountB, decimalsB)

      if (!needsWrapA && !needsWrapB) {
        const gasGate = evaluateProvideLiquidityCw20NativeGasGate(
          amountA,
          amountB,
          decimalsA,
          decimalsB,
          nativeUlunaQuery,
          provideCw20MinUlunaFees
        )
        if (!gasGate.canAddLiquidity) {
          throw new Error(gasGate.userMessage ?? 'Insufficient LUNC for gas')
        }
      }

      if (needsWrapA || needsWrapB) {
        if (needsWrapA && nativeEquivA) {
          rawA = (await netUlunaAfterTransferTaxAsync(BigInt(rawA), nativeEquivA)).toString()
        }
        if (needsWrapB && nativeEquivB) {
          rawB = (await netUlunaAfterTransferTaxAsync(BigInt(rawB), nativeEquivB)).toString()
        }
        const msgs: Array<{
          contract: string
          msg: Record<string, unknown>
          coins?: Array<{ denom: string; amount: string }>
        }> = []

        if (needsWrapA) {
          msgs.push({
            contract: TREASURY_CONTRACT_ADDRESS,
            msg: { wrap_deposit: {} },
            coins: [{ denom: nativeEquivA!, amount: rawA }],
          })
        }
        if (needsWrapB) {
          msgs.push({
            contract: TREASURY_CONTRACT_ADDRESS,
            msg: { wrap_deposit: {} },
            coins: [{ denom: nativeEquivB!, amount: rawB }],
          })
        }

        msgs.push({
          contract: tokenA,
          msg: {
            increase_allowance: {
              spender: pair.contract_addr,
              amount: rawA,
              expires: { never: {} },
            },
          },
        })
        msgs.push({
          contract: tokenB,
          msg: {
            increase_allowance: {
              spender: pair.contract_addr,
              amount: rawB,
              expires: { never: {} },
            },
          },
        })
        msgs.push({
          contract: pair.contract_addr,
          msg: {
            provide_liquidity: {
              assets: [
                { info: { token: { contract_addr: tokenA } }, amount: rawA },
                { info: { token: { contract_addr: tokenB } }, amount: rawB },
              ],
              slippage_tolerance: null,
              receiver: null,
              deadline: null,
            },
          },
        })

        return executeTerraContractMulti(address, msgs)
      }

      return provideLiquidity(address, pair.contract_addr, tokenA, tokenB, rawA, rawB)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setAmountA('')
      setAmountB('')
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['lpBalance', address, pair.liquidity_token] })
      queryClient.invalidateQueries({ queryKey: ['pool', pair.contract_addr] })
    },
    onError: () => sounds.playError(),
  })

  const removeMutation = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      const rawLp = toRawAmount(lpAmount, LP_DECIMALS)
      let minAssets: [string, string] | undefined
      if (poolQuery.data && withdrawSlippage) {
        const mins = withdrawMinAssetAmounts(
          rawLp,
          poolQuery.data.total_share,
          poolQuery.data.assets[0].amount,
          poolQuery.data.assets[1].amount,
          parseFloat(withdrawSlippage)
        )
        if (mins) minAssets = mins
      }
      const txHash = await withdrawLiquidity(address, pair.liquidity_token, pair.contract_addr, rawLp, minAssets)

      if (!receiveWrapped && WRAP_MAPPER_CONTRACT_ADDRESS) {
        const tokensToUnwrap: { cw20: string; denom: string }[] = []
        if (nativeEquivA) tokensToUnwrap.push({ cw20: tokenA, denom: nativeEquivA })
        if (nativeEquivB) tokensToUnwrap.push({ cw20: tokenB, denom: nativeEquivB })

        for (const { cw20 } of tokensToUnwrap) {
          const balanceRaw = await getTokenBalance(address, tokenAssetInfo(cw20))
          if (balanceRaw && balanceRaw !== '0') {
            const unwrapMsg = btoa(JSON.stringify({ unwrap: { recipient: null } }))
            await executeTerraContract(address, cw20, {
              send: {
                contract: WRAP_MAPPER_CONTRACT_ADDRESS,
                amount: balanceRaw,
                msg: unwrapMsg,
              },
            })
          }
        }
      }

      return txHash
    },
    onSuccess: () => {
      sounds.playSuccess()
      setLpAmount('')
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['lpBalance', address, pair.liquidity_token] })
      queryClient.invalidateQueries({ queryKey: ['pool', pair.contract_addr] })
    },
    onError: () => sounds.playError(),
  })

  return (
    <div className="shell-panel-strong">
      <div className="flex items-start justify-between mb-3">
        <div>
          <p
            className="font-medium uppercase tracking-wide flex items-center gap-1 font-heading"
            style={{ color: 'var(--ink)' }}
          >
            <TokenDisplay info={pair.asset_infos[0]} size={18} /> <span style={{ color: 'var(--ink-subtle)' }}>/</span>{' '}
            <TokenDisplay info={pair.asset_infos[1]} size={18} />
          </p>
          <p className="text-xs font-mono mt-1" style={{ color: 'var(--ink-subtle)' }}>
            {pairInfoMenuLabel(pair, { variant: 'full' })}
          </p>
          {volumeQuote24h && (
            <p className="text-xs mt-1 uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
              24h vol (quote, indexed): {formatNum(volumeQuote24h)}
            </p>
          )}
          {listBadges.isInFactoryRouterGraph ? (
            <span
              className="text-xs font-semibold px-2 py-0.5 rounded-none border"
              style={{
                color: 'var(--cyan)',
                borderColor: 'rgba(34, 211, 238, 0.4)',
                background: 'color-mix(in srgb, var(--cyan) 8%, transparent)',
              }}
              title="Pair contract is registered in the factory — included in the swap router’s on-chain token graph."
            >
              In router (factory)
            </span>
          ) : (
            <span
              className="text-xs font-semibold px-2 py-0.5 rounded-none border"
              style={{
                color: 'var(--color-negative)',
                borderColor: 'var(--color-negative)',
                background: 'color-mix(in srgb, var(--color-negative) 10%, transparent)',
              }}
              title="Not found in the factory’s pair list (indexer row only; not used for on-chain path finding until registered)."
            >
              Indexer only
            </span>
          )}
        </div>
        {feeQuery.data && (
          <span
            className="text-xs border-2 px-2 py-1 rounded-none shadow-[1px_1px_0_#000] uppercase tracking-wide font-semibold"
            style={{ color: 'var(--ink-dim)', borderColor: 'rgba(255,255,255,0.2)', background: 'var(--surface-0)' }}
          >
            Fee: <FeeDisplay feeBps={feeQuery.data.fee_bps} discountBps={discountQuery.data?.discount_bps} />
          </span>
        )}
      </div>

      {poolQuery.data && (
        <div className="flex gap-4 text-sm mb-4">
          <div className="flex-1 card-neo">
            <div className="mb-1">
              <TokenDisplay
                info={poolQuery.data.assets[0].info}
                size={14}
                className="text-xs font-semibold uppercase tracking-wide"
              />
            </div>
            <p className="font-mono text-xs" style={{ color: 'var(--ink)' }}>
              {formatTokenAmount(poolQuery.data.assets[0].amount, getDecimals(poolQuery.data.assets[0].info))}
            </p>
          </div>
          <div className="flex-1 card-neo">
            <div className="mb-1">
              <TokenDisplay
                info={poolQuery.data.assets[1].info}
                size={14}
                className="text-xs font-semibold uppercase tracking-wide"
              />
            </div>
            <p className="font-mono text-xs" style={{ color: 'var(--ink)' }}>
              {formatTokenAmount(poolQuery.data.assets[1].amount, getDecimals(poolQuery.data.assets[1].info))}
            </p>
          </div>
        </div>
      )}

      {poolQuery.isLoading && (
        <div className="flex items-center gap-2 text-xs mb-4" style={{ color: 'var(--ink-subtle)' }}>
          <Spinner size="sm" /> Loading pool...
        </div>
      )}
      {poolQuery.isError && (
        <RetryError
          message={
            isLcdConnectivityError(poolQuery.error) ? LCD_CONNECTIVITY_OUTAGE_MESSAGE : getErrorMessage(poolQuery.error)
          }
          onRetry={() => void poolQuery.refetch()}
        />
      )}

      <div className="flex gap-2 mb-3">
        <button
          onClick={() => {
            sounds.playButtonPress()
            setExpanded(expanded === 'add' ? null : 'add')
          }}
          className={`tab-neo !text-xs ${expanded === 'add' ? 'tab-neo-active' : 'tab-neo-inactive'}`}
        >
          Provide Liquidity
        </button>
        <button
          onClick={() => {
            sounds.playButtonPress()
            setExpanded(expanded === 'remove' ? null : 'remove')
          }}
          className={`tab-neo !text-xs ${expanded === 'remove' ? 'tab-neo-active' : 'tab-neo-inactive'}`}
        >
          Withdraw Liquidity
        </button>
      </div>

      {expanded === 'add' && (
        <div className="card-neo space-y-3 animate-fade-in-up">
          {tradingBlacklist.blocked && tradingBlacklist.message && (
            <p className="alert-error text-xs" role="alert">
              {tradingBlacklist.message}
            </p>
          )}
          <div>
            <label className="label-neo">
              Asset A Amount
              <span className="ml-1 normal-case" style={{ color: 'var(--ink-subtle)' }}>
                ({displayA.displayLabel})
              </span>
            </label>
            {hasNativeOptionA && (
              <label
                className="flex items-center gap-2 text-xs mb-1 cursor-pointer"
                style={{ color: 'var(--ink-dim)' }}
              >
                <input
                  type="checkbox"
                  checked={useNativeA}
                  onChange={(e) => setUseNativeA(e.target.checked)}
                  className="accent-[var(--cyan)]"
                />
                Use native {getTokenDisplaySymbol(nativeEquivA!)} (auto-wrap)
              </label>
            )}
            <input
              type="text"
              inputMode="decimal"
              value={amountA}
              onChange={(e) => {
                const v = e.target.value
                if (v === '' || /^\d*\.?\d*$/.test(v)) setAmountA(v)
              }}
              placeholder="0.00"
              className="input-neo"
              aria-label="Asset A amount"
            />
            {address && (
              <AmountBalanceActions
                balanceQuery={balanceAQuery}
                decimals={decimalsA}
                walletConnected={!!address}
                showHalf
                spendableRaw={maxResultA.spendableRaw}
                onMax={() => setAmountA(maxResultA.human)}
                onHalf={() => {
                  if (!balanceAQuery.data) return
                  const half = (BigInt(balanceAQuery.data) / 2n).toString()
                  setAmountA(fromRawAmount(half, decimalsA))
                }}
                testIdMax="pool-add-max-a"
                testIdHalf="pool-add-half-a"
              />
            )}
            {insufficientAddA && (
              <p className="text-xs font-semibold mt-1" style={{ color: 'var(--red, #ef4444)' }}>
                Exceeds wallet balance
              </p>
            )}
          </div>
          <div>
            <label className="label-neo">
              Asset B Amount
              <span className="ml-1 normal-case" style={{ color: 'var(--ink-subtle)' }}>
                ({displayB.displayLabel})
              </span>
            </label>
            {hasNativeOptionB && (
              <label
                className="flex items-center gap-2 text-xs mb-1 cursor-pointer"
                style={{ color: 'var(--ink-dim)' }}
              >
                <input
                  type="checkbox"
                  checked={useNativeB}
                  onChange={(e) => setUseNativeB(e.target.checked)}
                  className="accent-[var(--cyan)]"
                />
                Use native {getTokenDisplaySymbol(nativeEquivB!)} (auto-wrap)
              </label>
            )}
            <input
              type="text"
              inputMode="decimal"
              value={amountB}
              onChange={(e) => {
                const v = e.target.value
                if (v === '' || /^\d*\.?\d*$/.test(v)) setAmountB(v)
              }}
              placeholder="0.00"
              className="input-neo"
              aria-label="Asset B amount"
            />
            {address && (
              <AmountBalanceActions
                balanceQuery={balanceBQuery}
                decimals={decimalsB}
                walletConnected={!!address}
                showHalf
                spendableRaw={maxResultB.spendableRaw}
                onMax={() => setAmountB(maxResultB.human)}
                onHalf={() => {
                  if (!balanceBQuery.data) return
                  const half = (BigInt(balanceBQuery.data) / 2n).toString()
                  setAmountB(fromRawAmount(half, decimalsB))
                }}
                testIdMax="pool-add-max-b"
                testIdHalf="pool-add-half-b"
              />
            )}
            {insufficientAddB && (
              <p className="text-xs font-semibold mt-1" style={{ color: 'var(--red, #ef4444)' }}>
                Exceeds wallet balance
              </p>
            )}
          </div>
          {poolQuery.data && amountA && amountB && (
            <p className="text-sm" style={{ color: 'var(--ink-dim)' }} aria-live="polite" aria-atomic>
              {estimatedUserLp == null ? (
                <span>Estimated LP: — (amount too small or empty pool below minimum)</span>
              ) : (
                <span>Estimated LP: ~{formatTokenAmount(estimatedUserLp.toString(), LP_DECIMALS)} LP</span>
              )}
            </p>
          )}
          {poolQuery.data && amountA && amountB && ratioBalanced === false && (
            <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
              Amounts are not in the current pool price ratio. The contract mints LP from the smaller side; extra tokens
              on the larger side are effectively donated to the pool.
            </p>
          )}
          {provideLiquidityNativeGasGate.userMessage && (
            <p
              className="text-xs font-semibold"
              style={{
                color: provideLiquidityNativeGasGate.tone === 'warning' ? 'var(--ink-dim)' : 'var(--red, #ef4444)',
              }}
              role="alert"
            >
              {provideLiquidityNativeGasGate.userMessage}
            </p>
          )}
          <button
            onClick={() => {
              sounds.playButtonPress()
              addMutation.mutate()
            }}
            disabled={
              !address ||
              !amountA ||
              !amountB ||
              addMutation.isPending ||
              insufficientAdd ||
              !provideLiquidityNativeGasGate.canAddLiquidity ||
              tradingBlacklist.blocked
            }
            className={`w-full py-2.5 font-semibold text-sm ${
              !address ||
              !amountA ||
              !amountB ||
              addMutation.isPending ||
              insufficientAdd ||
              !provideLiquidityNativeGasGate.canAddLiquidity ||
              tradingBlacklist.blocked
                ? 'btn-disabled !w-full'
                : 'btn-primary !w-full'
            }`}
          >
            {!address
              ? 'Connect Wallet'
              : tradingBlacklist.blocked
                ? 'Trading restricted'
                : insufficientAdd
                  ? 'Insufficient balance'
                  : !provideLiquidityNativeGasGate.canAddLiquidity && provideLiquidityNativeGasGate.tone === 'warning'
                    ? 'Checking gas balance…'
                    : !provideLiquidityNativeGasGate.canAddLiquidity
                      ? 'Not enough LUNC for gas'
                      : terraBroadcastPendingButtonLabel(
                          addMutation.phase,
                          addMutation.isPending,
                          'Provide Liquidity',
                          'Providing Liquidity…'
                        )}
          </button>
          <TerraBroadcastPendingLink phase={addMutation.phase} txHash={addMutation.pendingTxHash} />
          {addMutation.isError && (
            <TxResultAlert type="error" message={addMutation.error?.message ?? 'Failed to provide liquidity'} />
          )}
          {addMutation.isSuccess && (
            <TxResultAlert type="success" message="Liquidity provided!" txHash={addMutation.data} />
          )}
        </div>
      )}

      {expanded === 'remove' && (
        <div className="card-neo space-y-3 animate-fade-in-up">
          {tradingBlacklist.blocked && tradingBlacklist.message && (
            <p className="alert-error text-xs" role="alert">
              {tradingBlacklist.message}
            </p>
          )}
          <div>
            <div className="flex items-center justify-between">
              <label className="label-neo" htmlFor={lpTokenAmountInputId}>
                LP Token Amount
              </label>
              {address && (
                <span className="text-xs" style={{ color: 'var(--ink-subtle)' }}>
                  Balance:{' '}
                  {lpBalanceQuery.isLoading ? (
                    <Spinner size="sm" />
                  ) : (
                    <button
                      type="button"
                      onClick={() => {
                        sounds.playButtonPress()
                        setLpAmount(fromRawAmount(lpBalance, LP_DECIMALS))
                      }}
                      className="font-mono underline cursor-pointer hover:opacity-80"
                      style={{ color: 'var(--cyan)' }}
                      title="Use max balance"
                    >
                      {lpBalanceDisplay}
                    </button>
                  )}
                </span>
              )}
            </div>
            <input
              id={lpTokenAmountInputId}
              type="text"
              inputMode="decimal"
              value={lpAmount}
              onChange={(e) => {
                const v = e.target.value
                if (v === '' || /^\d*\.?\d*$/.test(v)) setLpAmount(v)
              }}
              placeholder="0.00"
              className="input-neo"
            />
          </div>
          <p className="text-xs flex flex-wrap items-center gap-1" style={{ color: 'var(--ink-subtle)' }}>
            LP Token:{' '}
            <AddressRow
              address={pair.liquidity_token}
              startChars={8}
              endChars={6}
              copyAriaLabel="Copy LP token address"
              explorerAriaLabel="View LP token address on explorer"
              data-testid="pool-lp-token-address-row"
            />
          </p>
          {insufficientLp && (
            <p className="text-xs font-semibold" style={{ color: 'var(--red, #ef4444)' }}>
              Insufficient LP token balance
            </p>
          )}
          {(hasNativeOptionA || hasNativeOptionB) && (
            <label className="flex items-center gap-2 text-xs cursor-pointer" style={{ color: 'var(--ink-dim)' }}>
              <input
                type="checkbox"
                checked={receiveWrapped}
                onChange={(e) => setReceiveWrapped(e.target.checked)}
                className="accent-[var(--cyan)]"
              />
              Receive as wrapped tokens (uncheck to auto-unwrap to native)
            </label>
          )}
          <div>
            <label className="label-neo">Slippage Tolerance</label>
            <div className="flex gap-2">
              {['0.5', '1.0', '2.0'].map((val) => (
                <button
                  key={val}
                  onClick={() => {
                    sounds.playButtonPress()
                    setWithdrawSlippage(val)
                  }}
                  className={`tab-neo !text-xs !px-3 !py-1.5 ${
                    withdrawSlippage === val ? 'tab-neo-active' : 'tab-neo-inactive'
                  }`}
                >
                  {val}%
                </button>
              ))}
            </div>
          </div>
          <button
            onClick={() => {
              sounds.playButtonPress()
              removeMutation.mutate()
            }}
            disabled={!address || !lpAmount || insufficientLp || removeMutation.isPending || tradingBlacklist.blocked}
            className={`w-full py-2.5 font-semibold text-sm ${
              !address || !lpAmount || insufficientLp || removeMutation.isPending || tradingBlacklist.blocked
                ? 'btn-disabled !w-full'
                : 'btn-primary !w-full'
            }`}
          >
            {!address
              ? 'Connect Wallet'
              : tradingBlacklist.blocked
                ? 'Trading restricted'
                : insufficientLp
                  ? 'Insufficient LP Balance'
                  : terraBroadcastPendingButtonLabel(
                      removeMutation.phase,
                      removeMutation.isPending,
                      'Withdraw Liquidity',
                      'Withdrawing…'
                    )}
          </button>
          <TerraBroadcastPendingLink phase={removeMutation.phase} txHash={removeMutation.pendingTxHash} />
          {removeMutation.isError && (
            <TxResultAlert type="error" message={removeMutation.error?.message ?? 'Failed to withdraw liquidity'} />
          )}
          {removeMutation.isSuccess && (
            <TxResultAlert type="success" message="Liquidity withdrawn!" txHash={removeMutation.data} />
          )}
        </div>
      )}
    </div>
  )
})

const PAGE_SIZE = 20

export default function PoolPage() {
  const [q, setQ] = useState('')
  const [submittedQ, setSubmittedQ] = useState('')
  const [sort, setSort] = useState<IndexerPairSort>('symbol')
  const [order, setOrder] = useState<'asc' | 'desc'>('asc')
  const [page, setPage] = useState(0)
  const [routerKnownOnly, setRouterKnownOnly] = useState(false)

  const factoryPairsQuery = useQuery({
    queryKey: ['factoryPairsForPoolList', FACTORY_PAIRS_MAX_FOR_POOL_LIST] as const,
    queryFn: () => getAllPairsPaginated(FACTORY_PAIRS_MAX_FOR_POOL_LIST),
    staleTime: 60_000,
    enabled: !!FACTORY_CONTRACT_ADDRESS,
  })

  const factoryPairAddresses = useMemo(() => {
    const rows = factoryPairsQuery.data?.pairs ?? []
    return new Set(rows.map((p) => p.contract_addr))
  }, [factoryPairsQuery.data?.pairs])

  const pairsQuery = useQuery({
    queryKey: ['indexer-pairs', submittedQ, sort, order, page],
    queryFn: () =>
      getPairs({
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
        q: submittedQ.trim() || undefined,
        sort,
        order,
      }),
    staleTime: 30_000,
  })

  const indexerTokensQuery = useQuery({
    queryKey: ['indexer-tokens-list'],
    queryFn: getTokens,
    staleTime: 5 * 60_000,
    retry: false,
  })

  const total = pairsQuery.data?.total ?? 0
  const indexerPairs = useMemo(() => pairsQuery.data?.items ?? [], [pairsQuery.data?.items])
  const visibleIndexerPairs = useMemo(
    () => (routerKnownOnly ? indexerPairs.filter((ip) => factoryPairAddresses.has(ip.pair_address)) : indexerPairs),
    [indexerPairs, routerKnownOnly, factoryPairAddresses]
  )
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE))
  const canPrev = page > 0
  const canNext = (page + 1) * PAGE_SIZE < total

  const factoryCount = factoryPairsQuery.data?.pairs.length
  const showIndexerFactoryDrift =
    factoryPairsQuery.isSuccess && pairsQuery.isSuccess && factoryCount != null && factoryCount !== total

  return (
    <div className="max-w-4xl mx-auto">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 mb-6">
        <div>
          <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Liquidity Pools</h2>
          <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
            Browse pairs, compare fees, and add or remove liquidity.
          </p>
          <p className="text-xs mt-2 max-w-prose" style={{ color: 'var(--ink-dim)' }}>
            <span className="font-semibold" style={{ color: 'var(--ink-subtle)' }}>
              List source:{' '}
            </span>
            Count and sort order come from the{' '}
            <abbr
              title="Off-chain API used for pool metadata, 24h volume, and (elsewhere) route hints."
              className="no-underline border-b border-dotted border-white/20 cursor-help"
            >
              indexer
            </abbr>
            , not on-chain factory enumeration. Rows are tagged against the factory pair list (same set the swap router
            uses).{' '}
            <a
              href={POOL_DATA_SOURCES_DOC}
              target="_blank"
              rel="noreferrer"
              className="underline font-medium"
              style={{ color: 'var(--cyan)' }}
            >
              Data sources (docs)
            </a>
            .
          </p>
          {showIndexerFactoryDrift && (
            <p className="text-xs mt-2 font-medium" style={{ color: 'var(--ink-subtle)' }} role="status">
              Note: indexer reports {total.toLocaleString()} pair(s) while the factory currently lists {factoryCount}{' '}
              pair contract(s) — expect overlap but not a 1:1 count (indexing lag, allowlists, or cap on factory fetch).
            </p>
          )}
        </div>
        <div className="text-sm uppercase tracking-wide font-medium text-right" style={{ color: 'var(--ink-dim)' }}>
          <span className="block">{total.toLocaleString()} pair(s) (indexer total)</span>
          {indexerTokensQuery.data != null && (
            <span
              className="block text-xs font-normal mt-0.5 normal-case tracking-normal"
              title="Token metadata the indexer has recorded (for labels, volume, etc.); not the same as factory pair count."
            >
              {indexerTokensQuery.data.length} indexed tokens
            </span>
          )}
          {factoryPairsQuery.isSuccess && (
            <span
              className="block text-xs font-normal mt-0.5 normal-case tracking-normal"
              title="Pair contracts returned by factory `pairs` queries (up to the configured cap); used for router badges and filter."
            >
              {factoryCount?.toLocaleString() ?? '—'} on-chain (factory, router graph)
            </span>
          )}
          {factoryPairsQuery.isError && (
            <span className="block text-xs font-normal normal-case text-red-400 mt-0.5">Factory list unavailable</span>
          )}
        </div>
      </div>

      <div
        className="shell-panel mb-4 flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-end"
        role="search"
        aria-label="Filter and sort pools"
      >
        <div className="flex-1 min-w-[12rem]">
          <label htmlFor="pool-search" className="label-neo mb-1 block">
            Search
          </label>
          <div className="flex flex-col sm:flex-row gap-2">
            <input
              id="pool-search"
              type="search"
              className="input-neo flex-1"
              placeholder="Symbol, address, denom…"
              value={q}
              onChange={(e) => setQ(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  setPage(0)
                  setSubmittedQ(q)
                }
              }}
            />
            <button
              type="button"
              className="btn-muted shrink-0 sm:self-auto"
              onClick={() => {
                setPage(0)
                setSubmittedQ(q)
              }}
            >
              Search
            </button>
          </div>
        </div>
        <div>
          <label htmlFor="pool-sort" className="label-neo mb-1 block">
            Sort
          </label>
          <MenuSelect
            id="pool-sort"
            className="relative w-full sm:w-44"
            value={sort}
            options={POOL_SORT_OPTIONS}
            onChange={(v) => {
              const next = v as IndexerPairSort
              setSort(next)
              setPage(0)
              if (next === 'volume_24h') setOrder('desc')
            }}
          />
        </div>
        <div>
          <label htmlFor="pool-order" className="label-neo mb-1 block">
            Order
          </label>
          <MenuSelect
            id="pool-order"
            className="relative w-full sm:w-44"
            value={order}
            options={ORDER_OPTIONS}
            onChange={(v) => {
              setOrder(v as 'asc' | 'desc')
              setPage(0)
            }}
          />
        </div>
        <div className="flex items-center min-h-[2.5rem]">
          <label className="flex items-center gap-2 text-xs cursor-pointer" style={{ color: 'var(--ink-dim)' }}>
            <input
              type="checkbox"
              id="pool-filter-router"
              className="accent-[var(--cyan)]"
              checked={routerKnownOnly}
              onChange={(e) => {
                setRouterKnownOnly(e.target.checked)
                setPage(0)
              }}
            />
            <span>Router-known (factory) only on this page</span>
          </label>
        </div>
      </div>

      {pairsQuery.isLoading && (
        <div className="space-y-4" aria-live="polite">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} height="6rem" />
          ))}
        </div>
      )}

      {pairsQuery.isError && isIndexerUnavailableError(pairsQuery.error) && (
        <MarketDataServiceOutageBanner
          testId="pool-market-data-outage-banner"
          title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
          lead={POOL_MARKET_DATA_OUTAGE_LEAD}
          onRetry={() => void pairsQuery.refetch()}
        />
      )}

      {pairsQuery.isError && !isIndexerUnavailableError(pairsQuery.error) && (
        <RetryError
          message="Pool data is unavailable right now. Try again in a moment."
          onRetry={() => void pairsQuery.refetch()}
        />
      )}

      {!pairsQuery.isLoading && indexerPairs.length === 0 && !pairsQuery.isError && (
        <div className="shell-panel-strong py-8 text-center" style={{ color: 'var(--ink-dim)' }}>
          No liquidity pools match your filters.
        </div>
      )}

      {!pairsQuery.isLoading && indexerPairs.length > 0 && visibleIndexerPairs.length === 0 && !pairsQuery.isError && (
        <div className="shell-panel-strong py-8 text-center" style={{ color: 'var(--ink-dim)' }}>
          No pools on this page are in the factory router set. Try the next page or turn off the filter.
        </div>
      )}

      <div className="space-y-4">
        {visibleIndexerPairs.map((ip) => (
          <PoolCard
            key={ip.pair_address}
            pair={indexerPairToPairInfo(ip)}
            volumeQuote24h={ip.volume_quote_24h}
            listBadges={getPairListBadges({ pairAddress: ip.pair_address, factoryPairAddresses })}
          />
        ))}
      </div>

      {total > PAGE_SIZE && !pairsQuery.isLoading && !pairsQuery.isError && (
        <div className="flex items-center justify-between mt-6 pt-4 border-t border-white/10">
          <span className="text-xs uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
            Page {page + 1} of {totalPages}
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              className="btn-muted !text-xs"
              disabled={!canPrev}
              onClick={() => setPage((p) => Math.max(0, p - 1))}
            >
              Previous
            </button>
            <button
              type="button"
              className="btn-muted !text-xs"
              disabled={!canNext}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
