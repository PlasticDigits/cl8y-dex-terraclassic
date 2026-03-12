import { useState, memo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getPool, getFeeConfig, provideLiquidity, withdrawLiquidity } from '@/services/terraclassic/pair'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'
import type { PairInfo } from '@/types'
import { assetInfoLabel, tokenAssetInfo } from '@/types'
import { Spinner, TokenDisplay, RetryError, Skeleton } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'
import { formatTokenAmount, getDecimals } from '@/utils/formatAmount'

const PoolCard = memo(function PoolCard({ pair }: { pair: PairInfo }) {
  const address = useWalletStore((s) => s.address)
  const queryClient = useQueryClient()
  const [expanded, setExpanded] = useState<'add' | 'remove' | null>(null)
  const [amountA, setAmountA] = useState('')
  const [amountB, setAmountB] = useState('')
  const [lpAmount, setLpAmount] = useState('')

  const tokenA = assetInfoLabel(pair.asset_infos[0])
  const tokenB = assetInfoLabel(pair.asset_infos[1])
  const displayA = useTokenDisplayInfo(pair.asset_infos[0])
  const displayB = useTokenDisplayInfo(pair.asset_infos[1])

  const poolQuery = useQuery({
    queryKey: ['pool', pair.contract_addr],
    queryFn: () => getPool(pair.contract_addr),
    staleTime: 30_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', pair.contract_addr],
    queryFn: () => getFeeConfig(pair.contract_addr),
    staleTime: 60_000,
  })

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', address],
    queryFn: () => { if (!address) throw new Error('No address'); return getTraderDiscount(address) },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 15_000,
  })

  const lpBalanceQuery = useQuery({
    queryKey: ['lpBalance', address, pair.liquidity_token],
    queryFn: () => { if (!address) throw new Error('No address'); return getTokenBalance(address, tokenAssetInfo(pair.liquidity_token)) },
    enabled: !!address && expanded === 'remove',
    refetchInterval: 15_000,
  })

  const LP_DECIMALS = 6
  const lpBalance = lpBalanceQuery.data ?? '0'
  const lpBalanceDisplay = lpBalance === '0' ? '0' : formatTokenAmount(lpBalance, LP_DECIMALS)
  const insufficientLp = !!lpAmount && Number(lpAmount) * 10 ** LP_DECIMALS > Number(lpBalance)

  const addMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      return provideLiquidity(address, pair.contract_addr, tokenA, tokenB, amountA, amountB)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setAmountA('')
      setAmountB('')
    },
    onError: () => sounds.playError(),
  })

  const removeMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      return withdrawLiquidity(address, pair.liquidity_token, pair.contract_addr, lpAmount)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setLpAmount('')
      queryClient.invalidateQueries({ queryKey: ['lpBalance', address, pair.liquidity_token] })
      queryClient.invalidateQueries({ queryKey: ['pool', pair.contract_addr] })
    },
    onError: () => sounds.playError(),
  })

  return (
    <div className="shell-panel-strong">
      <div className="flex items-start justify-between mb-3">
        <div>
          <p className="font-medium uppercase tracking-wide flex items-center gap-1 font-heading" style={{ color: 'var(--ink)' }}>
            <TokenDisplay info={pair.asset_infos[0]} size={18} /> <span style={{ color: 'var(--ink-subtle)' }}>/</span> <TokenDisplay info={pair.asset_infos[1]} size={18} />
          </p>
          <p className="text-xs font-mono mt-1" style={{ color: 'var(--ink-subtle)' }}>
            Pair: {pair.contract_addr.slice(0, 10)}…{pair.contract_addr.slice(-6)}
          </p>
        </div>
        {feeQuery.data && (
          <span className="text-xs border-2 px-2 py-1 rounded-none shadow-[1px_1px_0_#000] uppercase tracking-wide font-semibold"
            style={{ color: 'var(--ink-dim)', borderColor: 'rgba(255,255,255,0.2)', background: 'var(--surface-0)' }}>
            Fee: {discountQuery.data && discountQuery.data.discount_bps > 0 ? (
              <>
                <span className="line-through mr-1">{feeQuery.data.fee_bps}</span>
                <span style={{ color: 'var(--cyan)' }}>
                  {Math.floor(feeQuery.data.fee_bps * (10000 - discountQuery.data.discount_bps) / 10000)}
                </span>
              </>
            ) : (
              feeQuery.data.fee_bps
            )} bps
          </span>
        )}
      </div>

      {poolQuery.data && (
        <div className="flex gap-4 text-sm mb-4">
          <div className="flex-1 card-neo">
            <div className="mb-1"><TokenDisplay info={poolQuery.data.assets[0].info} size={14} className="text-xs font-semibold uppercase tracking-wide" /></div>
            <p className="font-mono text-xs" style={{ color: 'var(--ink)' }}>{formatTokenAmount(poolQuery.data.assets[0].amount, getDecimals(poolQuery.data.assets[0].info))}</p>
          </div>
          <div className="flex-1 card-neo">
            <div className="mb-1"><TokenDisplay info={poolQuery.data.assets[1].info} size={14} className="text-xs font-semibold uppercase tracking-wide" /></div>
            <p className="font-mono text-xs" style={{ color: 'var(--ink)' }}>{formatTokenAmount(poolQuery.data.assets[1].amount, getDecimals(poolQuery.data.assets[1].info))}</p>
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
          className={`tab-neo !text-xs ${
            expanded === 'add' ? 'tab-neo-active' : 'tab-neo-inactive'
          }`}
        >
          Provide Liquidity
        </button>
        <button
          onClick={() => {
            sounds.playButtonPress()
            setExpanded(expanded === 'remove' ? null : 'remove')
          }}
          className={`tab-neo !text-xs ${
            expanded === 'remove' ? 'tab-neo-active' : 'tab-neo-inactive'
          }`}
        >
          Withdraw Liquidity
        </button>
      </div>

      {expanded === 'add' && (
        <div className="card-neo space-y-3 animate-fade-in-up">
          <div>
            <label className="label-neo">
              Asset A Amount
              <span className="ml-1 normal-case" style={{ color: 'var(--ink-subtle)' }}>({displayA.displayLabel})</span>
            </label>
            <input
              type="text"
              inputMode="decimal"
              value={amountA}
              onChange={(e) => setAmountA(e.target.value)}
              placeholder="0.00"
              className="input-neo"
            />
          </div>
          <div>
            <label className="label-neo">
              Asset B Amount
              <span className="ml-1 normal-case" style={{ color: 'var(--ink-subtle)' }}>({displayB.displayLabel})</span>
            </label>
            <input
              type="text"
              inputMode="decimal"
              value={amountB}
              onChange={(e) => setAmountB(e.target.value)}
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
              !address || !amountA || !amountB || addMutation.isPending
                ? 'btn-disabled !w-full'
                : 'btn-primary !w-full'
            }`}
          >
            {!address ? 'Connect Wallet' : addMutation.isPending ? 'Providing Liquidity...' : 'Provide Liquidity'}
          </button>
          {addMutation.isError && (
            <div className="alert-error">{addMutation.error?.message}</div>
          )}
          {addMutation.isSuccess && (
            <div className="alert-success">
              Liquidity provided! TX: <span className="font-mono text-xs">{addMutation.data}</span>
            </div>
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
                        setLpAmount(lpBalanceDisplay)
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
              onChange={(e) => setLpAmount(e.target.value)}
              placeholder="0.00"
              className="input-neo"
            />
          </div>
          <p className="text-xs" style={{ color: 'var(--ink-subtle)' }}>
            LP Token: <span className="font-mono">{pair.liquidity_token.slice(0, 10)}…{pair.liquidity_token.slice(-6)}</span>
          </p>
          {insufficientLp && (
            <p className="text-xs font-semibold" style={{ color: 'var(--red, #ef4444)' }}>
              Insufficient LP token balance
            </p>
          )}
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
            {!address ? 'Connect Wallet' : insufficientLp ? 'Insufficient LP Balance' : removeMutation.isPending ? 'Withdrawing...' : 'Withdraw Liquidity'}
          </button>
          {removeMutation.isError && (
            <div className="alert-error">{removeMutation.error?.message}</div>
          )}
          {removeMutation.isSuccess && (
            <div className="alert-success">
              Liquidity withdrawn! TX: <span className="font-mono text-xs">{removeMutation.data}</span>
            </div>
          )}
        </div>
      )}
    </div>
  )
})

export default function PoolPage() {
  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data?.pairs ?? []

  return (
    <div className="max-w-4xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Liquidity Pools</h2>
        <span className="text-sm uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>{pairs.length} pair(s)</span>
      </div>

      {pairsQuery.isLoading && (
        <div className="space-y-4" aria-live="polite">
          {Array.from({ length: 3 }).map((_, i) => <Skeleton key={i} height="6rem" />)}
        </div>
      )}

      {pairsQuery.isError && (
        <RetryError message={`Failed to load pools: ${pairsQuery.error?.message}`} onRetry={() => void pairsQuery.refetch()} />
      )}

      {!pairsQuery.isLoading && pairs.length === 0 && !pairsQuery.isError && (
        <div className="shell-panel-strong py-8 text-center" style={{ color: 'var(--ink-dim)' }}>
          No liquidity pools found.
        </div>
      )}

      <div className="space-y-4">
        {pairs.map((pair) => (
          <PoolCard key={pair.contract_addr} pair={pair} />
        ))}
      </div>
    </div>
  )
}
