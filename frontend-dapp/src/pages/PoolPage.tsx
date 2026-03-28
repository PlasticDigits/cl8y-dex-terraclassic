import { useState, memo, useMemo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getPool, provideLiquidity, withdrawLiquidity } from '@/services/terraclassic/pair'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTokenBalance, verifyPairInFactory } from '@/services/terraclassic/queries'
import { getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { executeTerraContract, executeTerraContractMulti } from '@/services/terraclassic/transactions'
import {
  FEE_DISCOUNT_CONTRACT_ADDRESS,
  FACTORY_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
  WRAP_MAPPER_CONTRACT_ADDRESS,
} from '@/utils/constants'
import type { PairInfo, AssetInfo } from '@/types'
import { assetInfoLabel, tokenAssetInfo, getNativeEquivalent, indexerPairToPairInfo } from '@/types'
import type { IndexerPairSort } from '@/types'
import { getPairs, getTokens, INDEXER_URL } from '@/services/indexer/client'
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
import { sounds } from '@/lib/sounds'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { formatTokenAmount, formatNum, getDecimals, toRawAmount, fromRawAmount } from '@/utils/formatAmount'

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

const PoolCard = memo(function PoolCard({ pair, volumeQuote24h }: { pair: PairInfo; volumeQuote24h?: string }) {
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

  const tokenA = assetInfoLabel(pair.asset_infos[0])
  const tokenB = assetInfoLabel(pair.asset_infos[1])
  const displayA = useTokenDisplayInfo(pair.asset_infos[0])
  const displayB = useTokenDisplayInfo(pair.asset_infos[1])

  const nativeEquivA = useMemo(() => getNativeEquivalent(tokenA), [tokenA])
  const nativeEquivB = useMemo(() => getNativeEquivalent(tokenB), [tokenB])
  const hasNativeOptionA = !!nativeEquivA
  const hasNativeOptionB = !!nativeEquivB

  const verifyQuery = useQuery({
    queryKey: ['pairVerify', pair.contract_addr],
    queryFn: () =>
      verifyPairInFactory(pair.contract_addr, FACTORY_CONTRACT_ADDRESS, pair.asset_infos as [AssetInfo, AssetInfo]),
    enabled: !!FACTORY_CONTRACT_ADDRESS,
    staleTime: Infinity,
  })

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

  const LP_DECIMALS = 6
  const lpBalance = lpBalanceQuery.data ?? '0'
  const lpBalanceDisplay = lpBalance === '0' ? '0' : formatTokenAmount(lpBalance, LP_DECIMALS)
  const lpRaw = Number(lpAmount)
  const insufficientLp = !!lpAmount && !isNaN(lpRaw) && lpRaw * 10 ** LP_DECIMALS > Number(lpBalance)

  const decimalsA = getDecimals(pair.asset_infos[0])
  const decimalsB = getDecimals(pair.asset_infos[1])

  const addMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      const rawA = toRawAmount(amountA, decimalsA)
      const rawB = toRawAmount(amountB, decimalsB)

      const needsWrapA = hasNativeOptionA && useNativeA
      const needsWrapB = hasNativeOptionB && useNativeB

      if (needsWrapA || needsWrapB) {
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
      queryClient.invalidateQueries({ queryKey: ['lpBalance'] })
      queryClient.invalidateQueries({ queryKey: ['pool', pair.contract_addr] })
    },
    onError: () => sounds.playError(),
  })

  const removeMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      const rawLp = toRawAmount(lpAmount, LP_DECIMALS)
      let minAssets: [string, string] | undefined
      if (poolQuery.data && withdrawSlippage) {
        const slippageFactor = 1 - parseFloat(withdrawSlippage) / 100
        const totalLp = parseFloat(poolQuery.data.total_share)
        const rawLpAmount = parseFloat(toRawAmount(lpAmount, LP_DECIMALS))
        if (totalLp > 0) {
          const shareRatio = rawLpAmount / totalLp
          const minA = Math.floor(parseFloat(poolQuery.data.assets[0].amount) * shareRatio * slippageFactor).toString()
          const minB = Math.floor(parseFloat(poolQuery.data.assets[1].amount) * shareRatio * slippageFactor).toString()
          minAssets = [minA, minB]
        }
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
            Pair: {pair.contract_addr.slice(0, 10)}…{pair.contract_addr.slice(-6)}
          </p>
          {volumeQuote24h && (
            <p className="text-xs mt-1 uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
              24h vol (quote, indexed): {formatNum(volumeQuote24h)}
            </p>
          )}
          {verifyQuery.data === false && (
            <span
              className="text-xs font-semibold px-2 py-0.5 rounded-none border"
              style={{
                color: 'var(--color-negative)',
                borderColor: 'var(--color-negative)',
                background: 'color-mix(in srgb, var(--color-negative) 10%, transparent)',
              }}
            >
              Unverified
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
            />
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
            />
          </div>
          <button
            onClick={() => {
              sounds.playButtonPress()
              addMutation.mutate()
            }}
            disabled={!address || !amountA || !amountB || addMutation.isPending}
            className={`w-full py-2.5 font-semibold text-sm ${
              !address || !amountA || !amountB || addMutation.isPending ? 'btn-disabled !w-full' : 'btn-primary !w-full'
            }`}
          >
            {!address ? 'Connect Wallet' : addMutation.isPending ? 'Providing Liquidity...' : 'Provide Liquidity'}
          </button>
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
          <div>
            <div className="flex items-center justify-between">
              <label className="label-neo">LP Token Amount</label>
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
          <p className="text-xs" style={{ color: 'var(--ink-subtle)' }}>
            LP Token:{' '}
            <span className="font-mono">
              {pair.liquidity_token.slice(0, 10)}…{pair.liquidity_token.slice(-6)}
            </span>
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
            disabled={!address || !lpAmount || insufficientLp || removeMutation.isPending}
            className={`w-full py-2.5 font-semibold text-sm ${
              !address || !lpAmount || insufficientLp || removeMutation.isPending
                ? 'btn-disabled !w-full'
                : 'btn-primary !w-full'
            }`}
          >
            {!address
              ? 'Connect Wallet'
              : insufficientLp
                ? 'Insufficient LP Balance'
                : removeMutation.isPending
                  ? 'Withdrawing...'
                  : 'Withdraw Liquidity'}
          </button>
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
  const indexerPairs = pairsQuery.data?.items ?? []
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE))
  const canPrev = page > 0
  const canNext = (page + 1) * PAGE_SIZE < total

  return (
    <div className="max-w-4xl mx-auto">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 mb-6">
        <div>
          <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Liquidity Pools</h2>
          <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
            Browse pairs, compare fees, and add or remove liquidity.
          </p>
        </div>
        <div className="text-sm uppercase tracking-wide font-medium text-right" style={{ color: 'var(--ink-dim)' }}>
          <span className="block">{total.toLocaleString()} pair(s)</span>
          {indexerTokensQuery.data != null && (
            <span className="block text-xs font-normal mt-0.5 normal-case tracking-normal">
              {indexerTokensQuery.data.length} indexed tokens
            </span>
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
      </div>

      {pairsQuery.isLoading && (
        <div className="space-y-4" aria-live="polite">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} height="6rem" />
          ))}
        </div>
      )}

      {pairsQuery.isError && (
        <RetryError
          message={`Pool data is unavailable right now. Check the indexer connection at ${INDEXER_URL} and try again.`}
          onRetry={() => void pairsQuery.refetch()}
        />
      )}

      {!pairsQuery.isLoading && indexerPairs.length === 0 && !pairsQuery.isError && (
        <div className="shell-panel-strong py-8 text-center" style={{ color: 'var(--ink-dim)' }}>
          No liquidity pools match your filters.
        </div>
      )}

      <div className="space-y-4">
        {indexerPairs.map((ip) => (
          <PoolCard key={ip.pair_address} pair={indexerPairToPairInfo(ip)} volumeQuote24h={ip.volume_quote_24h} />
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
