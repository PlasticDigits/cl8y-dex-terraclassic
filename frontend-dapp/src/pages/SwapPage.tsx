import { useState, useEffect, useCallback } from 'react'
import { useQuery, useMutation } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { simulateSwap, swap, getPool, getFeeConfig } from '@/services/terraclassic/pair'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { getTraderDiscount, getRegistration } from '@/services/terraclassic/feeDiscount'
import { FEE_DISCOUNT_CONTRACT_ADDRESS } from '@/utils/constants'
import { assetInfoLabel } from '@/types'
import { sounds } from '@/lib/sounds'
import { TokenDisplay } from '@/components/ui'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { formatTokenAmount, getDecimals } from '@/utils/formatAmount'

export default function SwapPage() {
  const address = useWalletStore((s) => s.address)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const { selectedPair, setSelectedPair, slippageTolerance, setSlippageTolerance } = useDexStore()

  const [inputAmount, setInputAmount] = useState('')
  const [isReversed, setIsReversed] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [customSlippage, setCustomSlippage] = useState('')

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data?.pairs ?? []

  useEffect(() => {
    if (pairs.length > 0 && !selectedPair) {
      setSelectedPair(pairs[0])
    }
  }, [pairsQuery.data, selectedPair, setSelectedPair, pairs])

  const offerAssetInfo = selectedPair
    ? isReversed
      ? selectedPair.asset_infos[1]
      : selectedPair.asset_infos[0]
    : null
  const receiveAssetInfo = selectedPair
    ? isReversed
      ? selectedPair.asset_infos[0]
      : selectedPair.asset_infos[1]
    : null

  const offerLabel = offerAssetInfo ? assetInfoLabel(offerAssetInfo) : ''
  const receiveLabel = receiveAssetInfo ? assetInfoLabel(receiveAssetInfo) : ''

  const poolQuery = useQuery({
    queryKey: ['pool', selectedPair?.contract_addr],
    queryFn: () => { if (!selectedPair) throw new Error('No pair'); return getPool(selectedPair.contract_addr) },
    enabled: !!selectedPair,
    refetchInterval: 15_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', selectedPair?.contract_addr],
    queryFn: () => { if (!selectedPair) throw new Error('No pair'); return getFeeConfig(selectedPair.contract_addr) },
    enabled: !!selectedPair,
  })

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', address],
    queryFn: () => { if (!address) throw new Error('No address'); return getTraderDiscount(address) },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 15_000,
  })

  const registrationQuery = useQuery({
    queryKey: ['feeDiscountRegistration', address],
    queryFn: () => { if (!address) throw new Error('No address'); return getRegistration(address) },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 15_000,
  })

  const balanceQuery = useQuery({
    queryKey: ['tokenBalance', address, offerLabel],
    queryFn: () => { if (!address || !offerAssetInfo) throw new Error('Missing params'); return getTokenBalance(address, offerAssetInfo) },
    enabled: !!address && !!offerAssetInfo,
    refetchInterval: 15_000,
  })

  const simQuery = useQuery({
    queryKey: ['simulation', selectedPair?.contract_addr, offerLabel, inputAmount],
    queryFn: () => { if (!selectedPair || !offerAssetInfo) throw new Error('Missing params'); return simulateSwap(selectedPair.contract_addr, offerAssetInfo, inputAmount) },
    enabled: !!selectedPair && !!offerAssetInfo && !!inputAmount && parseFloat(inputAmount) > 0,
    refetchInterval: 10_000,
  })

  const swapMutation = useMutation({
    mutationFn: async () => {
      if (!address || !selectedPair || !inputAmount || !offerAssetInfo) throw new Error('Missing parameters')
      const maxSpread = (slippageTolerance / 100).toString()
      return swap(address, offerLabel, selectedPair.contract_addr, inputAmount, undefined, maxSpread)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setInputAmount('')
    },
    onError: () => {
      sounds.playError()
    },
  })

  const outputAmount = simQuery.data?.return_amount ?? ''
  const commissionAmount = simQuery.data?.commission_amount ?? ''

  const priceImpact = simQuery.data
    ? (() => {
        const total =
          parseFloat(simQuery.data.return_amount) +
          parseFloat(simQuery.data.commission_amount) +
          parseFloat(simQuery.data.spread_amount)
        if (total === 0) return '0'
        return ((parseFloat(simQuery.data.spread_amount) / total) * 100).toFixed(2)
      })()
    : null

  const minReceived = simQuery.data
    ? Math.floor(parseFloat(simQuery.data.return_amount) * (1 - slippageTolerance / 100)).toString()
    : null

  const insufficientBalance =
    !!inputAmount &&
    parseFloat(inputAmount) > 0 &&
    balanceQuery.data !== undefined &&
    BigInt(inputAmount) > BigInt(balanceQuery.data)

  let buttonText = 'Swap'
  let buttonDisabled = false
  if (!isWalletConnected) {
    buttonText = 'Connect Wallet'
    buttonDisabled = true
  } else if (!selectedPair) {
    buttonText = 'Select a Pair'
    buttonDisabled = true
  } else if (!inputAmount || isNaN(parseFloat(inputAmount)) || parseFloat(inputAmount) <= 0) {
    buttonText = 'Enter Amount'
    buttonDisabled = true
  } else if (insufficientBalance) {
    buttonText = 'Insufficient Balance'
    buttonDisabled = true
  } else if (simQuery.isLoading) {
    buttonText = 'Calculating...'
    buttonDisabled = true
  } else if (swapMutation.isPending) {
    buttonText = 'Swapping...'
    buttonDisabled = true
  }

  const handlePairChange = useCallback((pairContract: string) => {
    const pair = pairs.find((p) => p.contract_addr === pairContract)
    if (pair) {
      sounds.playButtonPress()
      setSelectedPair(pair)
    }
  }, [pairs, setSelectedPair])

  const handleSlippagePreset = useCallback((value: number) => {
    sounds.playButtonPress()
    setSlippageTolerance(value)
    setCustomSlippage('')
  }, [setSlippageTolerance])

  const handleCustomSlippage = useCallback((value: string) => {
    setCustomSlippage(value)
    const parsed = parseFloat(value)
    if (!isNaN(parsed) && parsed > 0 && parsed <= 50) {
      setSlippageTolerance(parsed)
    }
  }, [setSlippageTolerance])

  return (
    <div className="max-w-[520px] mx-auto">
      <div className="relative">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-6 top-8 h-[78%] rounded-[28px] theme-hero-glow blur-2xl"
        />
        <div className="shell-panel-strong relative z-10">
          {/* Header */}
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Swap</h2>
            <button
              onClick={() => {
                sounds.playButtonPress()
                setShowSettings(!showSettings)
              }}
              className="btn-muted !text-xs !px-3 !py-1"
            >
              Settings
            </button>
          </div>

          {/* Slippage Settings */}
          {showSettings && (
            <div className="mb-6 card-neo animate-fade-in-up">
              <p className="label-neo mb-3">Slippage Tolerance</p>
              <div className="flex gap-2">
                {[0.1, 0.5, 1.0].map((val) => (
                  <button
                    key={val}
                    onClick={() => handleSlippagePreset(val)}
                    className={`tab-neo !text-xs !px-3 !py-1.5 ${
                      slippageTolerance === val && !customSlippage
                        ? 'tab-neo-active'
                        : 'tab-neo-inactive'
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
                    className="input-neo !text-xs !py-1.5"
                  />
                  <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs" style={{ color: 'var(--ink-subtle)' }}>%</span>
                </div>
              </div>
            </div>
          )}

          {/* Pair Selector */}
          <div className="mb-4">
            <label className="label-neo">Trading Pair</label>
            <select
              value={selectedPair?.contract_addr ?? ''}
              onChange={(e) => handlePairChange(e.target.value)}
              className="select-neo"
              aria-label="Select trading pair"
            >
              {pairs.length === 0 && <option value="">Loading pairs...</option>}
              {pairs.map((pair) => (
                <option key={pair.contract_addr} value={pair.contract_addr}>
                  {getTokenDisplaySymbol(assetInfoLabel(pair.asset_infos[0]))} / {getTokenDisplaySymbol(assetInfoLabel(pair.asset_infos[1]))}
                </option>
              ))}
            </select>
          </div>

          {/* You Pay */}
          <div className="card-neo mb-2">
            <div className="flex items-center justify-between mb-2">
              <span className="label-neo !mb-0">You Pay</span>
              <TokenDisplay info={offerAssetInfo} size={16} className="text-xs font-medium" />
            </div>
            <input
              type="text"
              inputMode="decimal"
              value={inputAmount}
              onChange={(e) => {
                const v = e.target.value
                if (v === '' || /^\d*\.?\d*$/.test(v)) setInputAmount(v)
              }}
              placeholder="0.00"
              className="w-full text-2xl font-medium bg-transparent focus:outline-none"
              style={{ color: 'var(--ink)' }}
            />
            {isWalletConnected && balanceQuery.data !== undefined && (
              <div className="flex items-center justify-between mt-2 text-xs" style={{ color: 'var(--ink-subtle)' }}>
                <span>
                  Balance: <span className="font-mono">{offerAssetInfo ? formatTokenAmount(balanceQuery.data!, getDecimals(offerAssetInfo)) : balanceQuery.data}</span>
                </span>
                <button
                  type="button"
                  onClick={() => {
                    sounds.playButtonPress()
                    setInputAmount(balanceQuery.data!)
                  }}
                  className="uppercase font-semibold tracking-wide hover:underline"
                  style={{ color: 'var(--cyan)' }}
                >
                  Max
                </button>
              </div>
            )}
          </div>

          {/* Swap Direction Toggle */}
          <div className="flex justify-center -my-1 relative z-10">
            <button
              onClick={() => {
                sounds.playButtonPress()
                setIsReversed(!isReversed)
              }}
              className="w-10 h-10 rounded-none border-2 flex items-center justify-center transition-all hover:translate-x-[1px] hover:translate-y-[1px] shadow-[2px_2px_0_#000] hover:shadow-[1px_1px_0_#000]"
              style={{
                borderColor: 'rgba(255,255,255,0.3)',
                background: 'var(--surface-raised)',
                color: 'var(--ink-dim)',
              }}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M8 1v14M8 1L4 5M8 1l4 4M8 15l-4-4M8 15l4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </button>
          </div>

          {/* You Receive */}
          <div className="card-neo mt-2 mb-4">
            <div className="flex items-center justify-between mb-2">
              <span className="label-neo !mb-0">You Receive</span>
              <TokenDisplay info={receiveAssetInfo} size={16} className="text-xs font-medium" />
            </div>
            <div className="text-2xl font-medium" style={{ color: 'var(--ink)' }}>
              {simQuery.isFetching ? (
                <span className="animate-pulse" style={{ color: 'var(--ink-subtle)' }}>Calculating...</span>
              ) : outputAmount && receiveAssetInfo ? (
                formatTokenAmount(outputAmount, getDecimals(receiveAssetInfo))
              ) : (
                <span style={{ color: 'var(--ink-subtle)' }}>0.00</span>
              )}
            </div>
          </div>

          {/* Trade Details */}
          {simQuery.data && (
            <div className="card-neo space-y-2 mb-4 text-sm">
              {poolQuery.data && (
                <div className="flex justify-between" style={{ color: 'var(--ink-dim)' }}>
                  <span className="uppercase text-xs tracking-wide font-medium">Pool Reserves</span>
                  <span className="font-mono text-xs">
                    {formatTokenAmount(poolQuery.data.assets[0].amount, getDecimals(poolQuery.data.assets[0].info))} / {formatTokenAmount(poolQuery.data.assets[1].amount, getDecimals(poolQuery.data.assets[1].info))}
                  </span>
                </div>
              )}
              {feeQuery.data && (
                <div className="flex justify-between" style={{ color: 'var(--ink-dim)' }}>
                  <span className="uppercase text-xs tracking-wide font-medium">Fee</span>
                  <span>
                    {discountQuery.data && discountQuery.data.discount_bps > 0 ? (
                      <>
                        <span className="line-through mr-1" style={{ color: 'var(--ink-subtle)' }}>
                          {(feeQuery.data.fee_bps / 100).toFixed(2)}%
                        </span>
                        <span style={{ color: 'var(--cyan)' }}>
                          {((feeQuery.data.fee_bps * (10000 - discountQuery.data.discount_bps)) / 10000 / 100).toFixed(2)}%
                        </span>
                        <span className="text-xs ml-1" style={{ color: 'var(--cyan)' }}>
                          (-{(discountQuery.data.discount_bps / 100).toFixed(0)}%)
                        </span>
                      </>
                    ) : (
                      <>{(feeQuery.data.fee_bps / 100).toFixed(2)}%</>
                    )}
                    {commissionAmount && receiveAssetInfo && <span className="ml-1" style={{ color: 'var(--ink-subtle)' }}>({formatTokenAmount(commissionAmount, getDecimals(receiveAssetInfo))})</span>}
                  </span>
                </div>
              )}
              {address && FEE_DISCOUNT_CONTRACT_ADDRESS && !registrationQuery.data?.registered && (
                <div className="p-2 border-2 rounded-none text-xs shadow-[1px_1px_0_#000]" style={{ borderColor: 'color-mix(in srgb, var(--cyan) 30%, transparent)', background: 'color-mix(in srgb, var(--cyan) 5%, transparent)', color: 'var(--cyan)' }}>
                  <a href="/tiers" className="hover:underline uppercase tracking-wide font-semibold">
                    Hold CL8Y to reduce swap fees &rarr;
                  </a>
                </div>
              )}
              {priceImpact !== null && (
                <>
                  <div className="flex justify-between" style={{ color: 'var(--ink-dim)' }}>
                    <span className="uppercase text-xs tracking-wide font-medium">Price Impact</span>
                    <span
                      className={
                        parseFloat(priceImpact) > 5
                          ? 'text-red-400 font-semibold'
                          : parseFloat(priceImpact) > 1
                            ? 'text-amber-400'
                            : 'text-green-400'
                      }
                    >
                      {priceImpact}%
                    </span>
                  </div>
                  {parseFloat(priceImpact) > 5 && (
                    <div className="alert-error !text-xs">
                      High price impact! You may receive significantly fewer tokens than expected.
                    </div>
                  )}
                </>
              )}
              {minReceived !== null && (
                <div className="flex justify-between" style={{ color: 'var(--ink-dim)' }}>
                  <span className="uppercase text-xs tracking-wide font-medium">Min Received</span>
                  <span className="font-mono text-xs">{receiveAssetInfo ? formatTokenAmount(minReceived!, getDecimals(receiveAssetInfo)) : minReceived}</span>
                </div>
              )}
              <div className="flex justify-between" style={{ color: 'var(--ink-dim)' }}>
                <span className="uppercase text-xs tracking-wide font-medium">Slippage Tolerance</span>
                <span>{slippageTolerance}%</span>
              </div>
            </div>
          )}

          {/* Swap Button */}
          <button
            onClick={() => {
              sounds.playButtonPress()
              swapMutation.mutate()
            }}
            disabled={buttonDisabled}
            className={`w-full py-4 font-semibold text-base ${
              buttonDisabled
                ? 'btn-disabled !w-full !py-4'
                : 'btn-primary btn-cta !w-full !py-4'
            }`}
          >
            {buttonText}
          </button>

          {swapMutation.isError && (
            <div className="mt-4 alert-error">
              {swapMutation.error?.message ?? 'Swap failed'}
            </div>
          )}

          {swapMutation.isSuccess && (
            <div className="mt-4 alert-success">
              Swap successful! TX: <span className="font-mono text-xs">{swapMutation.data}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
