import { useState } from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getAllPairs } from '@/services/terraclassic/factory'
import { getReserves, getFeeConfig, addLiquidity, removeLiquidity } from '@/services/terraclassic/pair'
import type { PairInfo } from '@/types'

function truncateAddr(addr: string): string {
  if (addr.length <= 16) return addr
  return `${addr.slice(0, 10)}...${addr.slice(-4)}`
}

function PoolCard({ pair }: { pair: PairInfo }) {
  const address = useWalletStore((s) => s.address)
  const [expanded, setExpanded] = useState<'add' | 'remove' | null>(null)
  const [amountA, setAmountA] = useState('')
  const [amountB, setAmountB] = useState('')
  const [lpAmount, setLpAmount] = useState('')

  const reservesQuery = useQuery({
    queryKey: ['reserves', pair.pair_contract],
    queryFn: () => getReserves(pair.pair_contract),
    staleTime: 30_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', pair.pair_contract],
    queryFn: () => getFeeConfig(pair.pair_contract),
    staleTime: 60_000,
  })

  const addMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      return addLiquidity(address, pair.pair_contract, pair.token_a, pair.token_b, amountA, amountB)
    },
    onSuccess: () => {
      setAmountA('')
      setAmountB('')
    },
  })

  const removeMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Wallet not connected')
      return removeLiquidity(address, pair.lp_token, pair.pair_contract, lpAmount)
    },
    onSuccess: () => setLpAmount(''),
  })

  return (
    <div className="bg-dex-card rounded-2xl border border-dex-border p-5">
      <div className="flex items-start justify-between mb-3">
        <div>
          <p className="font-medium text-white">
            {truncateAddr(pair.token_a)} / {truncateAddr(pair.token_b)}
          </p>
          <p className="text-xs text-gray-500 font-mono mt-1">
            Pair: {truncateAddr(pair.pair_contract)}
          </p>
        </div>
        {feeQuery.data && (
          <span className="text-xs text-gray-400 bg-dex-bg px-2 py-1 rounded-lg border border-dex-border">
            Fee: {feeQuery.data.fee_bps} bps
          </span>
        )}
      </div>

      {reservesQuery.data && (
        <div className="flex gap-4 text-sm text-gray-400 mb-4">
          <div className="flex-1 bg-dex-bg rounded-lg p-3 border border-dex-border">
            <p className="text-xs text-gray-500 mb-1">Reserve A</p>
            <p className="font-mono text-xs text-white">{reservesQuery.data.reserve_a}</p>
          </div>
          <div className="flex-1 bg-dex-bg rounded-lg p-3 border border-dex-border">
            <p className="text-xs text-gray-500 mb-1">Reserve B</p>
            <p className="font-mono text-xs text-white">{reservesQuery.data.reserve_b}</p>
          </div>
        </div>
      )}

      {reservesQuery.isLoading && (
        <div className="text-xs text-gray-500 mb-4 animate-pulse">Loading reserves...</div>
      )}

      <div className="flex gap-2 mb-3">
        <button
          onClick={() => setExpanded(expanded === 'add' ? null : 'add')}
          className={`text-sm px-4 py-1.5 rounded-lg transition-colors ${
            expanded === 'add'
              ? 'bg-dex-accent text-dex-bg'
              : 'border border-dex-border text-gray-300 hover:border-dex-accent/50'
          }`}
        >
          Add Liquidity
        </button>
        <button
          onClick={() => setExpanded(expanded === 'remove' ? null : 'remove')}
          className={`text-sm px-4 py-1.5 rounded-lg transition-colors ${
            expanded === 'remove'
              ? 'bg-dex-accent text-dex-bg'
              : 'border border-dex-border text-gray-300 hover:border-dex-accent/50'
          }`}
        >
          Remove Liquidity
        </button>
      </div>

      {expanded === 'add' && (
        <div className="space-y-3 p-4 rounded-xl bg-dex-bg border border-dex-border">
          <div>
            <label className="text-xs text-gray-400 mb-1 block">
              Token A Amount
              <span className="text-gray-600 ml-1">({truncateAddr(pair.token_a)})</span>
            </label>
            <input
              type="text"
              inputMode="decimal"
              value={amountA}
              onChange={(e) => setAmountA(e.target.value)}
              placeholder="0.00"
              className="w-full px-3 py-2 rounded-lg bg-dex-card border border-dex-border text-white text-sm focus:outline-none focus:border-dex-accent"
            />
          </div>
          <div>
            <label className="text-xs text-gray-400 mb-1 block">
              Token B Amount
              <span className="text-gray-600 ml-1">({truncateAddr(pair.token_b)})</span>
            </label>
            <input
              type="text"
              inputMode="decimal"
              value={amountB}
              onChange={(e) => setAmountB(e.target.value)}
              placeholder="0.00"
              className="w-full px-3 py-2 rounded-lg bg-dex-card border border-dex-border text-white text-sm focus:outline-none focus:border-dex-accent"
            />
          </div>
          <button
            onClick={() => addMutation.mutate()}
            disabled={!address || !amountA || !amountB || addMutation.isPending}
            className="w-full py-2.5 rounded-xl font-semibold text-sm transition-colors bg-dex-accent text-dex-bg hover:bg-dex-accent/80 disabled:bg-dex-border disabled:text-gray-500 disabled:cursor-not-allowed"
          >
            {!address ? 'Connect Wallet' : addMutation.isPending ? 'Adding Liquidity...' : 'Add Liquidity'}
          </button>
          {addMutation.isError && (
            <p className="text-red-400 text-sm">{addMutation.error?.message}</p>
          )}
          {addMutation.isSuccess && (
            <p className="text-green-400 text-sm">
              Liquidity added! TX: <span className="font-mono text-xs">{addMutation.data}</span>
            </p>
          )}
        </div>
      )}

      {expanded === 'remove' && (
        <div className="space-y-3 p-4 rounded-xl bg-dex-bg border border-dex-border">
          <div>
            <label className="text-xs text-gray-400 mb-1 block">LP Token Amount</label>
            <input
              type="text"
              inputMode="decimal"
              value={lpAmount}
              onChange={(e) => setLpAmount(e.target.value)}
              placeholder="0.00"
              className="w-full px-3 py-2 rounded-lg bg-dex-card border border-dex-border text-white text-sm focus:outline-none focus:border-dex-accent"
            />
          </div>
          <p className="text-xs text-gray-500">
            LP Token: <span className="font-mono">{truncateAddr(pair.lp_token)}</span>
          </p>
          <button
            onClick={() => removeMutation.mutate()}
            disabled={!address || !lpAmount || removeMutation.isPending}
            className="w-full py-2.5 rounded-xl font-semibold text-sm transition-colors bg-dex-accent text-dex-bg hover:bg-dex-accent/80 disabled:bg-dex-border disabled:text-gray-500 disabled:cursor-not-allowed"
          >
            {!address ? 'Connect Wallet' : removeMutation.isPending ? 'Removing...' : 'Remove Liquidity'}
          </button>
          {removeMutation.isError && (
            <p className="text-red-400 text-sm">{removeMutation.error?.message}</p>
          )}
          {removeMutation.isSuccess && (
            <p className="text-green-400 text-sm">
              Liquidity removed! TX: <span className="font-mono text-xs">{removeMutation.data}</span>
            </p>
          )}
        </div>
      )}
    </div>
  )
}

export default function PoolPage() {
  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairs(undefined, 50),
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data?.pairs ?? []

  return (
    <div className="max-w-4xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-lg font-semibold">Liquidity Pools</h2>
        <span className="text-sm text-gray-400">{pairs.length} pair(s)</span>
      </div>

      {pairsQuery.isLoading && (
        <div className="bg-dex-card rounded-2xl border border-dex-border p-8 text-center text-gray-400 animate-pulse">
          Loading pools...
        </div>
      )}

      {pairsQuery.isError && (
        <div className="bg-dex-card rounded-2xl border border-dex-border p-8 text-center text-red-400">
          Failed to load pools: {pairsQuery.error?.message}
        </div>
      )}

      {!pairsQuery.isLoading && pairs.length === 0 && !pairsQuery.isError && (
        <div className="bg-dex-card rounded-2xl border border-dex-border p-8 text-center text-gray-400">
          No liquidity pools found.
        </div>
      )}

      <div className="space-y-4">
        {pairs.map((pair) => (
          <PoolCard key={pair.pair_contract} pair={pair} />
        ))}
      </div>
    </div>
  )
}
