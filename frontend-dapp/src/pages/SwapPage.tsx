import { useState, useEffect, useCallback, useMemo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { simulateSwap, swap, getPool } from '@/services/terraclassic/pair'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { getTraderDiscount, getRegistration } from '@/services/terraclassic/feeDiscount'
import {
  findRoute,
  getAllTokens,
  simulateMultiHopSwap,
  executeMultiHopSwap,
  isDirectWrapUnwrap,
  findRouteWithNativeSupport,
  simulateNativeSwap,
  executeNativeSwap,
  type SwapOperation,
} from '@/services/terraclassic/router'
import { queryPausedState, checkRateLimitExceeded } from '@/services/terraclassic/wrapMapper'
import { FEE_DISCOUNT_CONTRACT_ADDRESS, WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import {
  assetInfoLabel,
  tokenAssetInfo,
  isNativeDenom,
  type HybridSwapParams,
  type IndexerRouteSolveResponse,
  type IndexerRouteQuoteKind,
} from '@/types'
import { sounds } from '@/lib/sounds'
import { FeeDisplay, TxResultAlert, TokenSelect, Spinner } from '@/components/ui'
import { pairInfoMenuLabel } from '@/utils/pairMenuOptions'
import { fetchCW20TokenInfo, getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'
import { formatTokenAmount, getDecimals, toRawAmount, fromRawAmount } from '@/utils/formatAmount'
import { getRouteSolve, postRouteSolve } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'

/** Wallet-side simulation result with optional indexer-routing metadata. */
interface SwapSimData {
  return_amount: string
  spread_amount: string
  commission_amount: string
  quoteDisclosure: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  indexerIntermediateTokens?: string[]
}

function quoteDisclosureForIndexerKind(kind: IndexerRouteQuoteKind | undefined): string {
  switch (kind) {
    case 'indexer_hybrid_lcd_degraded':
      return 'Indexer hybrid route (LCD) — one or more hops fell back to pool-only on the indexer.'
    case 'indexer_hybrid_lcd':
      return 'Indexer-optimized hybrid splits · quoted via your wallet LCD simulation (matches submit shape).'
    case 'indexer_pool_lcd':
      return 'Indexer route (pool-only legs) · quoted via your wallet LCD simulation.'
    case 'indexer_route_only':
      return 'Indexer-solved route · no aggregate router estimate (simulation unavailable).'
    default:
      return 'Quoted via your wallet (chain simulation).'
  }
}

export default function SwapPage() {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const { slippageTolerance, setSlippageTolerance, deadlineSeconds } = useDexStore()
  const queryClient = useQueryClient()

  const [inputAmount, setInputAmount] = useState('')
  const [fromToken, setFromToken] = useState<string>('')
  const [toToken, setToToken] = useState<string>('')
  const [showSettings, setShowSettings] = useState(false)
  const [customSlippage, setCustomSlippage] = useState('')
  const [showImpactConfirm, setShowImpactConfirm] = useState(false)
  const [indexerRouteResult, setIndexerRouteResult] = useState<IndexerRouteSolveResponse | null>(null)
  const [indexerRouteError, setIndexerRouteError] = useState<string | null>(null)
  const [indexerRouteLoading, setIndexerRouteLoading] = useState(false)
  const [useHybridBook, setUseHybridBook] = useState(false)
  const [bookInputHuman, setBookInputHuman] = useState('')
  const [hybridMaxMakers, setHybridMaxMakers] = useState(8)

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = useMemo(() => pairsQuery.data?.pairs ?? [], [pairsQuery.data])

  useEffect(() => {
    if (pairs.length > 0 && !fromToken) {
      const tokens = getAllTokens(pairs)
      if (tokens.length >= 2) {
        setFromToken(tokens[0])
        setToToken(tokens[1])
      }
    }
  }, [pairs, fromToken])

  const allTokens = useMemo(() => (pairs.length > 0 ? getAllTokens(pairs) : []), [pairs])

  useEffect(() => {
    const cw20Tokens = allTokens.filter((tokenId) => tokenId.startsWith('terra1'))
    cw20Tokens.forEach((tokenId) => {
      void fetchCW20TokenInfo(tokenId)
    })
  }, [allTokens])

  const wrapUnwrapType = fromToken && toToken ? isDirectWrapUnwrap(fromToken, toToken) : null
  const isWrapOrUnwrap = wrapUnwrapType !== null

  const nativeRouteInfo =
    fromToken && toToken && !isWrapOrUnwrap && (isNativeDenom(fromToken) || isNativeDenom(toToken))
      ? findRouteWithNativeSupport(pairs, fromToken, toToken)
      : null

  const route =
    fromToken && toToken && !isWrapOrUnwrap && !nativeRouteInfo ? findRoute(pairs, fromToken, toToken) : null
  const isDirect = route !== null && route.length === 1
  const isMultiHop = route !== null && route.length > 1
  const showHybridBookSubmitWarning = useMemo(() => {
    if (!isDirect || !useHybridBook || !fromToken.startsWith('terra1')) return false
    const t = bookInputHuman.trim()
    if (!t) return false
    const n = parseFloat(t)
    return !Number.isNaN(n) && n > 0
  }, [isDirect, useHybridBook, fromToken, bookInputHuman])

  /** CW20-only paths may be solvable via indexer hybrid graph even when the local pair list BFS misses (e.g. hop cap). */
  const indexerCw20Eligible =
    !!fromToken &&
    !!toToken &&
    fromToken.startsWith('terra1') &&
    toToken.startsWith('terra1') &&
    !isWrapOrUnwrap &&
    !nativeRouteInfo

  const hasRoute =
    isWrapOrUnwrap || nativeRouteInfo !== null || route !== null || indexerCw20Eligible

  const checkIndexerRoute = useCallback(async () => {
    if (!fromToken || !toToken) return
    setIndexerRouteError(null)
    setIndexerRouteResult(null)
    if (!fromToken.startsWith('terra1') || !toToken.startsWith('terra1')) {
      setIndexerRouteError(
        'Indexer route solver only accepts CW20 contract addresses in the asset table. Native-only denoms are not routable via this endpoint.'
      )
      return
    }
    setIndexerRouteLoading(true)
    try {
      const res = await getRouteSolve(fromToken, toToken)
      setIndexerRouteResult(res)
    } catch (e) {
      setIndexerRouteError(e instanceof Error ? e.message : String(e))
    } finally {
      setIndexerRouteLoading(false)
    }
  }, [fromToken, toToken])

  useEffect(() => {
    setIndexerRouteResult(null)
    setIndexerRouteError(null)
  }, [fromToken, toToken])

  const directPair = pairs.find((p) => {
    const a = assetInfoLabel(p.asset_infos[0])
    const b = assetInfoLabel(p.asset_infos[1])
    return (a === fromToken && b === toToken) || (b === fromToken && a === toToken)
  })

  const offerAssetInfo = fromToken ? tokenAssetInfo(fromToken) : null
  const receiveAssetInfo = toToken ? tokenAssetInfo(toToken) : null

  const poolQuery = useQuery({
    queryKey: ['pool', directPair?.contract_addr],
    queryFn: () => {
      if (!directPair) throw new Error('No pair')
      return getPool(directPair.contract_addr)
    },
    enabled: !!directPair,
    refetchInterval: 15_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', directPair?.contract_addr],
    queryFn: () => {
      if (!directPair) throw new Error('No pair')
      return getPairFeeConfig(directPair.contract_addr)
    },
    enabled: !!directPair,
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

  const registrationQuery = useQuery({
    queryKey: ['feeDiscountRegistration', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getRegistration(address)
    },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 15_000,
  })

  const balanceQuery = useQuery({
    queryKey: ['tokenBalance', address, fromToken],
    queryFn: () => {
      if (!address || !offerAssetInfo) throw new Error('Missing params')
      return getTokenBalance(address, offerAssetInfo)
    },
    enabled: !!address && !!offerAssetInfo,
    refetchInterval: 15_000,
  })

  const offerDecimals = offerAssetInfo ? getDecimals(offerAssetInfo) : 6
  const rawInputAmount = inputAmount ? toRawAmount(inputAmount, offerDecimals) : '0'

  const needsWrapCheck = isWrapOrUnwrap ? wrapUnwrapType === 'wrap' : (nativeRouteInfo?.needsWrapInput ?? false)
  const wrapDenom = needsWrapCheck ? (isNativeDenom(fromToken) ? fromToken : null) : null

  const pausedQuery = useQuery({
    queryKey: ['wrapMapperPaused'],
    queryFn: queryPausedState,
    enabled:
      !!WRAP_MAPPER_CONTRACT_ADDRESS &&
      (needsWrapCheck ||
        (isWrapOrUnwrap && wrapUnwrapType === 'unwrap') ||
        (nativeRouteInfo?.needsUnwrapOutput ?? false)),
    staleTime: 30_000,
  })

  const rateLimitQuery = useQuery({
    queryKey: ['rateLimit', wrapDenom, rawInputAmount],
    queryFn: () => {
      if (!wrapDenom) throw new Error('No denom')
      return checkRateLimitExceeded(wrapDenom, rawInputAmount)
    },
    enabled: !!wrapDenom && !!rawInputAmount && rawInputAmount !== '0',
    staleTime: 15_000,
  })

  const isWrapPaused = pausedQuery.data === true
  const isRateLimitExceeded = rateLimitQuery.data === true

  const simQuery = useQuery({
    queryKey: [
      'simulation',
      fromToken,
      toToken,
      rawInputAmount,
      JSON.stringify(route),
      wrapUnwrapType,
      JSON.stringify(nativeRouteInfo),
      useHybridBook,
      bookInputHuman,
      hybridMaxMakers,
    ],
    queryFn: async (): Promise<SwapSimData> => {
      if (!inputAmount || parseFloat(inputAmount) <= 0) throw new Error('Missing params')

      if (isWrapOrUnwrap) {
        return {
          return_amount: rawInputAmount,
          spread_amount: '0',
          commission_amount: '0',
          quoteDisclosure: 'Wrap / unwrap (1:1).',
        }
      }

      if (nativeRouteInfo) {
        const result = await simulateNativeSwap(rawInputAmount, fromToken, toToken, pairs)
        return {
          return_amount: result.amount,
          spread_amount: '0',
          commission_amount: '0',
          quoteDisclosure: 'Native / wrapped route · wallet LCD simulation (pool-only ops).',
        }
      }

      // Advanced: manual limit-book split on a direct pair (overrides indexer hybrid for this quote).
      if (isDirect && directPair) {
        if (useHybridBook && fromToken.startsWith('terra1')) {
          const payDec = getDecimals(tokenAssetInfo(fromToken))
          const bookRaw = bookInputHuman.trim() ? toRawAmount(bookInputHuman.trim(), payDec) : '0'
          const total = BigInt(rawInputAmount)
          const book = BigInt(bookRaw)
          if (book > 0n) {
            if (book > total) throw new Error('Book leg cannot exceed pay amount')
            if (hybridMaxMakers < 1) throw new Error('max maker fills must be at least 1')
            const pool = total - book
            try {
              const idx = await postRouteSolve(fromToken, toToken, rawInputAmount, [
                {
                  pool_input: pool.toString(),
                  book_input: book.toString(),
                  max_maker_fills: hybridMaxMakers,
                  book_start_hint: null,
                },
              ])
              if (idx.estimated_amount_out?.trim()) {
                return {
                  return_amount: idx.estimated_amount_out,
                  spread_amount: '0',
                  commission_amount: '0',
                  quoteDisclosure:
                    'Manual hybrid split (Settings) · indexer merged ops + router LCD estimate.',
                  indexerQuoteKind: idx.quote_kind,
                }
              }
            } catch {
              /* fall through */
            }
          }
        }
      }

      // Default CW20↔CW20: indexer hybrid optimization (≤3 hops) + wallet `simulate_swap_operations`.
      if (
        fromToken.startsWith('terra1') &&
        toToken.startsWith('terra1') &&
        rawInputAmount !== '0'
      ) {
        try {
          const idx = await getRouteSolve(fromToken, toToken, rawInputAmount, {
            hybridOptimize: true,
            maxMakerFills: hybridMaxMakers,
          })
          const tin = idx.token_in.trim().toLowerCase()
          const tout = idx.token_out.trim().toLowerCase()
          if (tin === fromToken.trim().toLowerCase() && tout === toToken.trim().toLowerCase()) {
            const ops = swapOperationsFromIndexerResponse(
              idx.router_operations as unknown[],
              idx.hops.length
            )
            const result = await simulateMultiHopSwap(rawInputAmount, ops)
            const intermediates =
              idx.intermediate_tokens?.length === idx.hops.length + 1
                ? idx.intermediate_tokens
                : [idx.token_in, ...idx.hops.map((h) => h.ask_token)]
            return {
              return_amount: result.amount,
              spread_amount: '0',
              commission_amount: '0',
              quoteDisclosure: quoteDisclosureForIndexerKind(idx.quote_kind),
              indexerQuoteKind: idx.quote_kind,
              indexerOperations: ops,
              indexerIntermediateTokens: intermediates,
            }
          }
        } catch {
          /* pool-only fallback */
        }
      }

      if (!route) throw new Error('No route found')
      if (isDirect && directPair) {
        const offerInfo = tokenAssetInfo(fromToken)
        const r = await simulateSwap(directPair.contract_addr, offerInfo, rawInputAmount)
        return {
          ...r,
          quoteDisclosure: 'Direct pool · on-chain pair simulation (pool-only).',
        }
      }
      if (isMultiHop && route) {
        const result = await simulateMultiHopSwap(rawInputAmount, route)
        return {
          return_amount: result.amount,
          spread_amount: '0',
          commission_amount: '0',
          quoteDisclosure:
            'Client-discovered multihop (pair graph) · pool-only · wallet LCD simulation.',
        }
      }
      throw new Error('No route found')
    },
    enabled: hasRoute && !!inputAmount && parseFloat(inputAmount) > 0,
    refetchInterval: 10_000,
  })

  const swapMutation = useMutation({
    mutationFn: async () => {
      if (!address || !inputAmount) throw new Error('Missing parameters')
      const maxSpread = (slippageTolerance / 100).toString()

      if (isWrapOrUnwrap || nativeRouteInfo) {
        const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
        return executeNativeSwap(
          address,
          fromToken,
          toToken,
          rawInputAmount,
          pairs,
          maxSpread,
          minReceived ?? undefined,
          deadline
        )
      }

      const idxOps = simQuery.data?.indexerOperations
      if (idxOps && idxOps.length > 0) {
        const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
        return executeMultiHopSwap(
          address,
          fromToken,
          rawInputAmount,
          idxOps,
          maxSpread,
          minReceived ?? undefined,
          undefined,
          deadline
        )
      }

      if (!route) throw new Error('No route found')

      if (isDirect && directPair) {
        const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
        let hybrid: HybridSwapParams | undefined
        if (useHybridBook && fromToken.startsWith('terra1')) {
          const payDec = getDecimals(tokenAssetInfo(fromToken))
          const bookRaw = bookInputHuman.trim() ? toRawAmount(bookInputHuman.trim(), payDec) : '0'
          const total = BigInt(rawInputAmount)
          const book = BigInt(bookRaw)
          if (book > total) throw new Error('Book leg cannot exceed pay amount')
          if (book > 0n && hybridMaxMakers < 1) throw new Error('max maker fills must be at least 1')
          if (book > 0n) {
            const pool = total - book
            hybrid = {
              pool_input: pool.toString(),
              book_input: book.toString(),
              max_maker_fills: hybridMaxMakers,
              book_start_hint: null,
            }
          }
        }
        return swap(address, fromToken, directPair.contract_addr, rawInputAmount, undefined, maxSpread, undefined, {
          hybrid,
          deadline,
        })
      }

      if (isMultiHop && route) {
        const minReceive = minReceived ?? undefined
        return executeMultiHopSwap(address, fromToken, rawInputAmount, route, maxSpread, minReceive)
      }

      throw new Error('No route found')
    },
    onSuccess: () => {
      sounds.playSuccess()
      setInputAmount('')
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['pool'] })
      queryClient.invalidateQueries({ queryKey: ['simulation'] })
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
    BigInt(rawInputAmount) > BigInt(balanceQuery.data)

  let buttonText = 'Swap'
  let buttonDisabled = false
  if (!isWalletConnected) {
    buttonText = 'Connect Wallet'
    buttonDisabled = false
  } else if (!hasRoute) {
    buttonText = 'No Route'
    buttonDisabled = true
  } else if (isWrapPaused) {
    buttonText = 'Wrapping is Temporarily Paused'
    buttonDisabled = true
  } else if (!inputAmount || isNaN(parseFloat(inputAmount)) || parseFloat(inputAmount) <= 0) {
    buttonText = 'Enter Amount'
    buttonDisabled = true
  } else if (isRateLimitExceeded) {
    buttonText = 'Rate Limit Exceeded'
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
  } else if (showImpactConfirm) {
    buttonText = `Confirm Swap (${priceImpact}% impact)`
    buttonDisabled = false
  }

  const handleSlippagePreset = useCallback(
    (value: number) => {
      sounds.playButtonPress()
      setSlippageTolerance(value)
      setCustomSlippage('')
    },
    [setSlippageTolerance]
  )

  const handleCustomSlippage = useCallback(
    (value: string) => {
      // Block non-numeric input: only allow digits and one decimal point
      const sanitized = value.replace(/[^\d.]/g, '').replace(/(\.\d*)\./g, '$1') // keep only first decimal (e.g. "5.5.5" -> "5.55")
      setCustomSlippage(sanitized)
      const parsed = parseFloat(sanitized)
      if (!isNaN(parsed) && parsed >= 0.01 && parsed <= 50) {
        setSlippageTolerance(parsed)
      } else if (!isNaN(parsed) && parsed > 50) {
        setSlippageTolerance(50)
      }
    },
    [setSlippageTolerance]
  )

  const customSlippageError =
    customSlippage !== '' &&
    (isNaN(parseFloat(customSlippage)) || parseFloat(customSlippage) < 0.01 || parseFloat(customSlippage) > 50)

  const handleToggleSettings = useCallback(() => {
    sounds.playButtonPress()
    setShowSettings((prev) => !prev)
  }, [])

  const handleOpenSettings = useCallback(() => {
    sounds.playButtonPress()
    setShowSettings(true)
  }, [])

  return (
    <div className="relative max-w-[620px] mx-auto w-full">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-3 sm:inset-x-6 top-6 sm:top-8 h-[78%] rounded-[40px] theme-hero-glow blur-3xl"
      />
      <div className="shell-panel-strong relative z-10 !p-5 sm:!p-6">
        {/* Header */}
        <div className="flex items-center justify-between gap-2 mb-4 sm:mb-6">
          <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Swap</h2>
          <button onClick={handleToggleSettings} className="btn-muted !text-[11px] sm:!text-xs !px-2.5 sm:!px-3 !py-1">
            Settings
          </button>
        </div>

        {/* Slippage Settings */}
        {showSettings && (
          <>
            <div id="swap-slippage-settings" className="mb-4 sm:mb-6 card-neo animate-fade-in-up">
              <p className="label-neo mb-3">Slippage Tolerance</p>
              <div className="flex flex-wrap gap-2">
                {[0.1, 0.5, 1.0].map((val) => (
                  <button
                    key={val}
                    onClick={() => handleSlippagePreset(val)}
                    className={`tab-neo !text-xs !px-3 !py-1.5 ${
                      slippageTolerance === val && !customSlippage ? 'tab-neo-active' : 'tab-neo-inactive'
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
                  <span
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-xs"
                    style={{ color: 'var(--ink-subtle)' }}
                  >
                    %
                  </span>
                </div>
              </div>
              {customSlippageError && (
                <p
                  className="mt-2 text-xs font-semibold uppercase tracking-wide"
                  style={{ color: 'var(--color-negative)' }}
                >
                  Must be between 0.01% and 50%
                </p>
              )}
              {!customSlippageError && slippageTolerance > 5 && (
                <p
                  className="mt-2 text-xs font-semibold uppercase tracking-wide"
                  style={{ color: 'var(--color-warning, #f59e0b)' }}
                >
                  High slippage increases front-running risk
                </p>
              )}
            </div>
            {showSettings && isDirect && !isWrapOrUnwrap && directPair && (
              <div className="mb-4 sm:mb-6 card-neo animate-fade-in-up">
                <p className="label-neo mb-2">Advanced — direct swap: limit book leg</p>
                <p className="text-[10px] font-mono mb-2" style={{ color: 'var(--ink-subtle)' }}>
                  {pairInfoMenuLabel(directPair, { variant: 'full' })}
                </p>
                <p className="text-[10px] mb-3 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
                  Only single-hop CW20 swaps. The estimate above is pool-only; Pattern C execution can differ when the
                  on-chain book has liquidity.
                </p>
                <label className="flex items-center gap-2 text-xs mb-3 cursor-pointer">
                  <input type="checkbox" checked={useHybridBook} onChange={(e) => setUseHybridBook(e.target.checked)} />
                  Route part of input through the limit book
                </label>
                {useHybridBook && (
                  <div className="space-y-2">
                    <div>
                      <label className="label-neo text-[10px]">
                        Book leg amount ({getTokenDisplaySymbol(fromToken)})
                      </label>
                      <input
                        className="input-neo !text-xs w-full"
                        value={bookInputHuman}
                        onChange={(e) => setBookInputHuman(e.target.value)}
                        placeholder="0.0"
                      />
                    </div>
                    <div>
                      <label className="label-neo text-[10px]">Max distinct makers</label>
                      <input
                        type="number"
                        className="input-neo !text-xs w-full"
                        min={1}
                        max={256}
                        value={hybridMaxMakers}
                        onChange={(e) => setHybridMaxMakers(Number(e.target.value) || 8)}
                      />
                    </div>
                  </div>
                )}
              </div>
            )}
            <div className="mb-4 sm:mb-6 card-neo animate-fade-in-up">
              <p className="label-neo mb-3">Indexer route check</p>
              <p className="text-[10px] mb-3 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
                Compares this token pair with the indexer&apos;s BFS graph (max 4 hops). Only CW20 addresses present in
                the indexer asset table are supported; native-only assets without a CW20 row are not routable via{' '}
                <code className="font-mono text-[10px]">/api/v1/route/solve</code>.
              </p>
              <button
                type="button"
                className="btn-muted !text-xs"
                onClick={() => {
                  sounds.playButtonPress()
                  void checkIndexerRoute()
                }}
                disabled={indexerRouteLoading || !fromToken || !toToken}
              >
                {indexerRouteLoading ? 'Checking…' : 'Compare indexer route'}
              </button>
              {indexerRouteError && (
                <p className="text-xs mt-2 font-medium" style={{ color: 'var(--color-negative)' }}>
                  {indexerRouteError}
                </p>
              )}
              {indexerRouteResult && (
                <div className="mt-3 text-[11px] space-y-1.5 font-mono" style={{ color: 'var(--ink-subtle)' }}>
                  <p>
                    Indexer hops: {indexerRouteResult.hops.length}
                    {route != null && <span style={{ color: 'var(--ink-dim)' }}> · Client hops: {route.length}</span>}
                  </p>
                  {indexerRouteResult.hops.map((h, i) => (
                    <p key={`${h.pair}-${i}`}>
                      {i + 1}. {shortenAddress(h.pair, 8, 6)} · {shortenAddress(h.offer_token, 4, 4)} →{' '}
                      {shortenAddress(h.ask_token, 4, 4)}
                    </p>
                  ))}
                </div>
              )}
            </div>
          </>
        )}

        {/* You Pay / swap direction / You Receive — abutting cards, control on seam */}
        <div className="swap-io-stack relative mb-4">
          <div className="card-neo swap-io-card-pay !p-4 sm:!p-5">
            <div className="flex flex-col gap-2 mb-3 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
              <span className="label-neo !mb-0 sm:pt-1">You Pay</span>
              <TokenSelect
                value={fromToken}
                tokens={allTokens}
                excludeToken={toToken}
                onChange={(tokenId) => {
                  sounds.playButtonPress()
                  setFromToken(tokenId)
                  setShowImpactConfirm(false)
                }}
                aria-label="Select token you pay"
                disabled={allTokens.length === 0}
              />
            </div>
            <input
              type="text"
              inputMode="decimal"
              value={inputAmount}
              onChange={(e) => {
                const v = e.target.value
                if (v === '' || /^\d*\.?\d*$/.test(v)) {
                  setInputAmount(v)
                  setShowImpactConfirm(false)
                }
              }}
              placeholder="0.00"
              className="w-full text-[1.75rem] sm:text-2xl font-medium bg-transparent focus:outline-none"
              style={{ color: 'var(--ink)' }}
            />
            {isWalletConnected && (
              <div
                className="flex flex-wrap items-center justify-between gap-2 mt-2 text-xs min-h-[1.5rem]"
                style={{ color: 'var(--ink-subtle)' }}
              >
                <span className="inline-flex items-center gap-1.5 min-w-0 max-w-full">
                  <span className="shrink-0">Balance:</span>
                  {!offerAssetInfo ? (
                    <span className="font-mono">—</span>
                  ) : balanceQuery.isLoading ? (
                    <span className="inline-flex items-center" aria-busy="true" aria-live="polite">
                      <Spinner size="sm" className="!w-3.5 !h-3.5 opacity-90" />
                      <span className="sr-only">Loading balance</span>
                    </span>
                  ) : balanceQuery.isError ? (
                    <span className="font-mono">—</span>
                  ) : (
                    <span className="font-mono truncate">
                      {formatTokenAmount(balanceQuery.data ?? '0', getDecimals(offerAssetInfo))}
                    </span>
                  )}
                </span>
                <button
                  type="button"
                  disabled={!offerAssetInfo || balanceQuery.isLoading || balanceQuery.isError || !balanceQuery.data}
                  onClick={() => {
                    sounds.playButtonPress()
                    if (balanceQuery.data) setInputAmount(fromRawAmount(balanceQuery.data, offerDecimals))
                  }}
                  className="ml-auto uppercase font-semibold tracking-wide hover:underline shrink-0 disabled:opacity-40 disabled:cursor-not-allowed disabled:no-underline"
                  style={{ color: 'var(--cyan)' }}
                >
                  Max
                </button>
              </div>
            )}
          </div>

          <div className="relative z-20 flex justify-center pointer-events-none -my-[22px] sm:-my-6">
            <button
              type="button"
              aria-label="Swap pay and receive tokens"
              onClick={() => {
                sounds.playButtonPress()
                const tmp = fromToken
                setFromToken(toToken)
                setToToken(tmp)
                setShowImpactConfirm(false)
              }}
              className="pointer-events-auto w-11 h-11 sm:w-12 sm:h-12 rounded-[18px] border flex items-center justify-center transition-all hover:-translate-y-0.5"
              style={{
                borderColor: 'rgba(255, 225, 190, 0.2)',
                background:
                  'linear-gradient(180deg, rgba(72, 44, 31, 0.98), rgba(37, 22, 18, 0.99)), rgba(255, 255, 255, 0.03)',
                color: 'var(--cyan)',
                boxShadow:
                  '0 16px 34px rgba(0, 0, 0, 0.24), 0 0 0 1px rgba(255, 161, 59, 0.08), inset 0 1px 0 rgba(255, 243, 221, 0.2)',
              }}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden>
                <path
                  d="M8 1v14M8 1L4 5M8 1l4 4M8 15l-4-4M8 15l4-4"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </button>
          </div>

          <div className="card-neo swap-io-card-receive !p-4 sm:!p-5">
            <div className="flex flex-col gap-2 mb-3 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
              <span className="label-neo !mb-0 sm:pt-1">You Receive</span>
              <TokenSelect
                value={toToken}
                tokens={allTokens}
                excludeToken={fromToken}
                onChange={(tokenId) => {
                  sounds.playButtonPress()
                  setToToken(tokenId)
                  setShowImpactConfirm(false)
                }}
                aria-label="Select token you receive"
                disabled={allTokens.length === 0}
              />
            </div>
            <div className="text-[1.75rem] sm:text-2xl font-medium" style={{ color: 'var(--ink)' }}>
              {simQuery.isFetching ? (
                <span className="animate-pulse" style={{ color: 'var(--ink-subtle)' }}>
                  Calculating...
                </span>
              ) : outputAmount && receiveAssetInfo ? (
                formatTokenAmount(outputAmount, getDecimals(receiveAssetInfo))
              ) : (
                <span style={{ color: 'var(--ink-subtle)' }}>0.00</span>
              )}
            </div>
          </div>
        </div>

        <div className="mb-4 space-y-2">
          {simQuery.data?.quoteDisclosure && (
            <div className="card-neo text-[11px] sm:text-xs leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
              <span className="uppercase tracking-wide font-semibold" style={{ color: 'var(--ink-subtle)' }}>
                Quote source:{' '}
              </span>
              {simQuery.data.quoteDisclosure}
            </div>
          )}
          {simQuery.data?.indexerIntermediateTokens && simQuery.data.indexerIntermediateTokens.length >= 2 && (
            <div className="card-neo text-xs break-words leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
              <span className="uppercase tracking-wide font-medium">Route (indexer): </span>
              {simQuery.data.indexerIntermediateTokens.map((t, i) => (
                <span key={`${t}-${i}`}>
                  {i > 0 && ' → '}
                  {getTokenDisplaySymbol(t)}
                </span>
              ))}
            </div>
          )}
          {isWrapOrUnwrap && (
            <div className="card-neo text-xs break-words leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
              This swap will {wrapUnwrapType === 'wrap' ? 'wrap' : 'unwrap'} your {getTokenDisplaySymbol(fromToken)}{' '}
              (1:1)
            </div>
          )}
          {nativeRouteInfo && (
            <div className="card-neo text-xs break-words leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
              <span className="uppercase tracking-wide font-medium">Route: </span>
              {nativeRouteInfo.needsWrapInput && <span>{getTokenDisplaySymbol(fromToken)} → </span>}
              {nativeRouteInfo.operations.map((op, i) => (
                <span key={i}>
                  {i > 0 && ' → '}
                  {getTokenDisplaySymbol(assetInfoLabel(op.terra_swap.offer_asset_info))}
                </span>
              ))}
              {' → '}
              {getTokenDisplaySymbol(
                assetInfoLabel(
                  nativeRouteInfo.operations[nativeRouteInfo.operations.length - 1].terra_swap.ask_asset_info
                )
              )}
              {nativeRouteInfo.needsUnwrapOutput && <span> → {getTokenDisplaySymbol(toToken)}</span>}
              {(nativeRouteInfo.needsWrapInput || nativeRouteInfo.needsUnwrapOutput) && (
                <div className="mt-1">
                  This swap will{' '}
                  {nativeRouteInfo.needsWrapInput && nativeRouteInfo.needsUnwrapOutput
                    ? 'wrap and unwrap'
                    : nativeRouteInfo.needsWrapInput
                      ? 'wrap'
                      : 'unwrap'}{' '}
                  your tokens
                </div>
              )}
            </div>
          )}
          {isMultiHop && route && (
            <div className="card-neo text-xs break-words leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
              <span className="uppercase tracking-wide font-medium">Route: </span>
              {route.map((op, i) => (
                <span key={i}>
                  {i > 0 && ' → '}
                  {getTokenDisplaySymbol(assetInfoLabel(op.terra_swap.offer_asset_info))}
                </span>
              ))}
              {' → '}
              {getTokenDisplaySymbol(toToken)}
            </div>
          )}
          {fromToken && toToken && !hasRoute && (
            <div className="alert-error !text-xs">No route found between these tokens</div>
          )}
        </div>

        {/* Trade Details */}
        {simQuery.data && (
          <div className="card-neo mb-4 grid grid-cols-2 gap-x-3 gap-y-2 text-xs sm:text-sm sm:block sm:space-y-2">
            {poolQuery.data && (
              <div
                className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                style={{ color: 'var(--ink-dim)' }}
              >
                <span className="uppercase text-xs tracking-wide font-medium">Pool Reserves</span>
                <span className="font-mono text-xs sm:text-right break-all">
                  {formatTokenAmount(poolQuery.data.assets[0].amount, getDecimals(poolQuery.data.assets[0].info))} /{' '}
                  {formatTokenAmount(poolQuery.data.assets[1].amount, getDecimals(poolQuery.data.assets[1].info))}
                </span>
              </div>
            )}
            {feeQuery.data && (
              <div
                className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                style={{ color: 'var(--ink-dim)' }}
              >
                <span className="uppercase text-xs tracking-wide font-medium">Fee</span>
                <FeeDisplay
                  feeBps={feeQuery.data.fee_bps}
                  discountBps={discountQuery.data?.discount_bps}
                  commissionAmount={
                    commissionAmount && receiveAssetInfo
                      ? formatTokenAmount(commissionAmount, getDecimals(receiveAssetInfo))
                      : undefined
                  }
                />
              </div>
            )}
            {address && FEE_DISCOUNT_CONTRACT_ADDRESS && !registrationQuery.data?.registered && (
              <div
                className="col-span-2 p-2 border-2 rounded-none text-xs shadow-[1px_1px_0_#000]"
                style={{
                  borderColor: 'color-mix(in srgb, var(--cyan) 30%, transparent)',
                  background: 'color-mix(in srgb, var(--cyan) 5%, transparent)',
                  color: 'var(--cyan)',
                }}
              >
                <a href="/tiers" className="hover:underline uppercase tracking-wide font-semibold">
                  Hold CL8Y to reduce swap fees &rarr;
                </a>
              </div>
            )}
            {priceImpact !== null && (
              <>
                <div
                  className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                  style={{ color: 'var(--ink-dim)' }}
                >
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
                  <div className="col-span-2 alert-error !text-xs">
                    High price impact! You may receive significantly fewer tokens than expected.
                  </div>
                )}
              </>
            )}
            {minReceived !== null && (
              <div
                className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                style={{ color: 'var(--ink-dim)' }}
              >
                <span className="uppercase text-xs tracking-wide font-medium">Min Received</span>
                <span className="font-mono text-xs sm:text-right break-all">
                  {receiveAssetInfo ? formatTokenAmount(minReceived!, getDecimals(receiveAssetInfo)) : minReceived}
                </span>
              </div>
            )}
            <button
              type="button"
              onClick={handleOpenSettings}
              aria-expanded={showSettings}
              aria-controls="swap-slippage-settings"
              className="min-w-0 flex flex-col gap-1 text-left sm:flex-row sm:items-start sm:justify-between"
              style={{ color: 'var(--ink-dim)' }}
            >
              <span className="uppercase text-xs tracking-wide font-medium">Slippage Tolerance</span>
              <span className="inline-flex items-center gap-1">
                <span>{slippageTolerance}%</span>
                <span aria-hidden="true" className="text-[10px]" style={{ color: 'var(--cyan)' }}>
                  {showSettings ? '▲' : '▼'}
                </span>
              </span>
            </button>
          </div>
        )}

        {(showHybridBookSubmitWarning ||
          simQuery.data?.indexerQuoteKind === 'indexer_hybrid_lcd' ||
          simQuery.data?.indexerQuoteKind === 'indexer_hybrid_lcd_degraded') && (
          <div className="alert-error mb-3 text-xs" role="alert">
            <p className="font-semibold mb-1">Limit book leg</p>
            <p>
              The on-screen estimate may still diverge from execution if the indexer or LCD snapshot differs from the
              chain at submit time (hybrid / L8). Read{' '}
              <a
                href="https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/limit-orders.md"
                target="_blank"
                rel="noopener noreferrer"
                className="underline font-mono text-[10px]"
              >
                docs/limit-orders.md
              </a>{' '}
              for integrator semantics.
            </p>
          </div>
        )}
        {/* Swap Button */}
        {showImpactConfirm && (
          <div className="alert-error mb-3 text-xs">
            <p className="font-semibold mb-1">High Price Impact Warning</p>
            <p>
              This trade has a {priceImpact}% price impact. You may receive significantly fewer tokens than expected.
              Click the button again to confirm.
            </p>
          </div>
        )}
        <button
          onClick={() => {
            sounds.playButtonPress()
            if (!isWalletConnected) {
              openWalletModal()
              return
            }
            if (priceImpact && parseFloat(priceImpact) > 5 && !showImpactConfirm) {
              setShowImpactConfirm(true)
              return
            }
            setShowImpactConfirm(false)
            swapMutation.mutate()
          }}
          disabled={buttonDisabled}
          className={`w-full py-3.5 sm:py-4 font-semibold text-base ${
            buttonDisabled ? 'btn-disabled !w-full !py-3.5 sm:!py-4' : 'btn-primary btn-cta !w-full !py-3.5 sm:!py-4'
          }`}
        >
          {buttonText}
        </button>

        {swapMutation.isError && (
          <div className="mt-4">
            <TxResultAlert type="error" message={swapMutation.error?.message ?? 'Swap failed'} />
          </div>
        )}

        {swapMutation.isSuccess && (
          <div className="mt-4">
            <TxResultAlert type="success" message="Swap successful!" txHash={swapMutation.data} />
          </div>
        )}
      </div>
    </div>
  )
}
