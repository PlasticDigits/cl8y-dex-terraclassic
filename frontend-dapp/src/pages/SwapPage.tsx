import { useState, useEffect } from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { getAllPairs } from '@/services/terraclassic/factory'
import { simulateSwap, swap, getReserves, getFeeConfig } from '@/services/terraclassic/pair'

function truncateAddr(addr: string): string {
  if (addr.length <= 16) return addr
  return `${addr.slice(0, 10)}...${addr.slice(-4)}`
}

export default function SwapPage() {
  const address = useWalletStore((s) => s.address)
  const { selectedPair, setSelectedPair, slippageTolerance, setSlippageTolerance } = useDexStore()

  const [inputAmount, setInputAmount] = useState('')
  const [isReversed, setIsReversed] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [customSlippage, setCustomSlippage] = useState('')

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairs(undefined, 50),
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data?.pairs ?? []

  useEffect(() => {
    if (pairs.length > 0 && !selectedPair) {
      setSelectedPair(pairs[0])
    }
  }, [pairsQuery.data, selectedPair, setSelectedPair, pairs])

  const offerToken = selectedPair
    ? isReversed ? selectedPair.token_b : selectedPair.token_a
    : ''
  const receiveToken = selectedPair
    ? isReversed ? selectedPair.token_a : selectedPair.token_b
    : ''

  const reservesQuery = useQuery({
    queryKey: ['reserves', selectedPair?.pair_contract],
    queryFn: () => getReserves(selectedPair!.pair_contract),
    enabled: !!selectedPair,
    refetchInterval: 15_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', selectedPair?.pair_contract],
    queryFn: () => getFeeConfig(selectedPair!.pair_contract),
    enabled: !!selectedPair,
  })

  const simQuery = useQuery({
    queryKey: ['simulateSwap', selectedPair?.pair_contract, offerToken, inputAmount],
    queryFn: () => simulateSwap(selectedPair!.pair_contract, offerToken, inputAmount),
    enabled: !!selectedPair && !!inputAmount && parseFloat(inputAmount) > 0,
    refetchInterval: 10_000,
  })

  const swapMutation = useMutation({
    mutationFn: async () => {
      if (!address || !selectedPair || !inputAmount) throw new Error('Missing parameters')
      const minOutput = simQuery.data
        ? Math.floor(parseFloat(simQuery.data.return_amount) * (1 - slippageTolerance / 100)).toString()
        : undefined
      return swap(address, offerToken, selectedPair.pair_contract, inputAmount, minOutput)
    },
    onSuccess: () => setInputAmount(''),
  })

  const outputAmount = simQuery.data?.return_amount ?? ''
  const feeAmount = simQuery.data?.fee_amount ?? ''

  const priceImpact = simQuery.data
    ? (() => {
        const total =
          parseFloat(simQuery.data.return_amount) +
          parseFloat(simQuery.data.fee_amount) +
          parseFloat(simQuery.data.spread_amount)
        if (total === 0) return '0'
        return ((parseFloat(simQuery.data.spread_amount) / total) * 100).toFixed(2)
      })()
    : null

  const minReceived = simQuery.data
    ? Math.floor(parseFloat(simQuery.data.return_amount) * (1 - slippageTolerance / 100)).toString()
    : null

  let buttonText = 'Swap'
  let buttonDisabled = false
  if (!address) {
    buttonText = 'Connect Wallet'
    buttonDisabled = true
  } else if (!selectedPair) {
    buttonText = 'Select a Pair'
    buttonDisabled = true
  } else if (!inputAmount || parseFloat(inputAmount) <= 0) {
    buttonText = 'Enter Amount'
    buttonDisabled = true
  } else if (simQuery.isLoading) {
    buttonText = 'Calculating...'
    buttonDisabled = true
  } else if (swapMutation.isPending) {
    buttonText = 'Swapping...'
    buttonDisabled = true
  }

  function handlePairChange(pairContract: string) {
    const pair = pairs.find((p) => p.pair_contract === pairContract)
    if (pair) setSelectedPair(pair)
  }

  function handleSlippagePreset(value: number) {
    setSlippageTolerance(value)
    setCustomSlippage('')
  }

  function handleCustomSlippage(value: string) {
    setCustomSlippage(value)
    const parsed = parseFloat(value)
    if (!isNaN(parsed) && parsed > 0 && parsed <= 50) {
      setSlippageTolerance(parsed)
    }
  }

  return (
    <div className="max-w-lg mx-auto">
      <div className="bg-dex-card rounded-2xl border border-dex-border p-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-lg font-semibold">Swap</h2>
          <button
            onClick={() => setShowSettings(!showSettings)}
            className="text-gray-400 hover:text-white transition-colors text-sm"
          >
            Settings
          </button>
        </div>

        {/* Slippage Settings */}
        {showSettings && (
          <div className="mb-6 p-4 rounded-xl bg-dex-bg border border-dex-border">
            <p className="text-sm text-gray-400 mb-3">Slippage Tolerance</p>
            <div className="flex gap-2">
              {[0.1, 0.5, 1.0].map((val) => (
                <button
                  key={val}
                  onClick={() => handleSlippagePreset(val)}
                  className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                    slippageTolerance === val && !customSlippage
                      ? 'bg-dex-accent text-dex-bg'
                      : 'bg-dex-card border border-dex-border text-gray-300 hover:border-dex-accent/50'
                  }`}
                >
                  {val}%
                </button>
              ))}
              <div className="relative flex-1">
                <input
                  type="text"
                  value={customSlippage}
                  onChange={(e) => handleCustomSlippage(e.target.value)}
                  placeholder="Custom"
                  className="w-full px-3 py-1.5 rounded-lg text-sm bg-dex-card border border-dex-border text-white placeholder-gray-500 focus:outline-none focus:border-dex-accent"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-sm text-gray-500">%</span>
              </div>
            </div>
          </div>
        )}

        {/* Pair Selector */}
        <div className="mb-4">
          <label className="text-xs text-gray-400 mb-1 block">Trading Pair</label>
          <select
            value={selectedPair?.pair_contract ?? ''}
            onChange={(e) => handlePairChange(e.target.value)}
            className="w-full px-4 py-2.5 rounded-xl bg-dex-bg border border-dex-border text-white text-sm focus:outline-none focus:border-dex-accent appearance-none cursor-pointer"
          >
            {pairs.length === 0 && <option value="">Loading pairs...</option>}
            {pairs.map((pair) => (
              <option key={pair.pair_contract} value={pair.pair_contract}>
                {truncateAddr(pair.token_a)} / {truncateAddr(pair.token_b)}
              </option>
            ))}
          </select>
        </div>

        {/* You Pay */}
        <div className="rounded-xl bg-dex-bg border border-dex-border p-4 mb-2">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs text-gray-400">You Pay</span>
            <span className="text-xs text-gray-500 font-mono">
              {offerToken ? truncateAddr(offerToken) : '--'}
            </span>
          </div>
          <input
            type="text"
            inputMode="decimal"
            value={inputAmount}
            onChange={(e) => setInputAmount(e.target.value)}
            placeholder="0.00"
            className="w-full text-2xl font-medium bg-transparent text-white placeholder-gray-600 focus:outline-none"
          />
        </div>

        {/* Swap Direction Toggle */}
        <div className="flex justify-center -my-1 relative z-10">
          <button
            onClick={() => setIsReversed(!isReversed)}
            className="w-10 h-10 rounded-xl bg-dex-card border-4 border-dex-bg flex items-center justify-center text-gray-400 hover:text-dex-accent transition-colors"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className="rotate-0">
              <path d="M8 1v14M8 1L4 5M8 1l4 4M8 15l-4-4M8 15l4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
        </div>

        {/* You Receive */}
        <div className="rounded-xl bg-dex-bg border border-dex-border p-4 mt-2 mb-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs text-gray-400">You Receive</span>
            <span className="text-xs text-gray-500 font-mono">
              {receiveToken ? truncateAddr(receiveToken) : '--'}
            </span>
          </div>
          <div className="text-2xl font-medium text-white">
            {simQuery.isFetching ? (
              <span className="text-gray-500 animate-pulse">Calculating...</span>
            ) : outputAmount ? (
              outputAmount
            ) : (
              <span className="text-gray-600">0.00</span>
            )}
          </div>
        </div>

        {/* Trade Details */}
        {simQuery.data && (
          <div className="space-y-2 mb-4 text-sm border border-dex-border rounded-xl p-4">
            {reservesQuery.data && (
              <div className="flex justify-between text-gray-400">
                <span>Pool Reserves</span>
                <span className="font-mono text-xs">
                  {reservesQuery.data.reserve_a} / {reservesQuery.data.reserve_b}
                </span>
              </div>
            )}
            {feeQuery.data && (
              <div className="flex justify-between text-gray-400">
                <span>Fee</span>
                <span>
                  {feeQuery.data.fee_bps / 100}%
                  {feeAmount && <span className="text-gray-500 ml-1">({feeAmount})</span>}
                </span>
              </div>
            )}
            {priceImpact !== null && (
              <div className="flex justify-between text-gray-400">
                <span>Price Impact</span>
                <span
                  className={
                    parseFloat(priceImpact) > 5
                      ? 'text-red-400'
                      : parseFloat(priceImpact) > 1
                        ? 'text-amber-400'
                        : 'text-green-400'
                  }
                >
                  {priceImpact}%
                </span>
              </div>
            )}
            {minReceived !== null && (
              <div className="flex justify-between text-gray-400">
                <span>Min Received</span>
                <span className="font-mono text-xs">{minReceived}</span>
              </div>
            )}
            <div className="flex justify-between text-gray-400">
              <span>Slippage Tolerance</span>
              <span>{slippageTolerance}%</span>
            </div>
          </div>
        )}

        {/* Swap Button */}
        <button
          onClick={() => swapMutation.mutate()}
          disabled={buttonDisabled}
          className={`w-full py-4 rounded-xl font-semibold text-base transition-colors ${
            buttonDisabled
              ? 'bg-dex-border text-gray-500 cursor-not-allowed'
              : 'bg-dex-accent text-dex-bg hover:bg-dex-accent/80'
          }`}
        >
          {buttonText}
        </button>

        {swapMutation.isError && (
          <div className="mt-4 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
            {swapMutation.error?.message ?? 'Swap failed'}
          </div>
        )}

        {swapMutation.isSuccess && (
          <div className="mt-4 p-3 rounded-xl bg-green-500/10 border border-green-500/20 text-green-400 text-sm">
            Swap successful! TX: <span className="font-mono text-xs">{swapMutation.data}</span>
          </div>
        )}
      </div>
    </div>
  )
}
