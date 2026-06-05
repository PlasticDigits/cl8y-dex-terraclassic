import { useEffect, useMemo, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'

import { placeLimitOrderBatchWithAllowance, placeLimitOrderLadderWithAllowance } from '@/services/terraclassic/pair'
import { getPairLimitPlacements } from '@/services/indexer/client'
import { TxResultAlert } from '@/components/ui'
import { useLimitOrderConfig } from '@/hooks/useLimitOrderConfig'
import { useLimitOrderForm } from '@/hooks/useLimitOrderForm'
import { useLimitLadderPlaceGates } from '@/hooks/useLimitLadderPlaceGates'
import { useLimitLadderPlacementPlan } from '@/hooks/useLimitLadderPlacementPlan'
import { useTradeBestBookPrices } from '@/hooks/useTradeBestBookPrices'
import { LimitOrderAdvancedLimitSettings } from '@/components/trade/LimitOrderAdvancedLimitSettings'
import { LimitOrderBidAskSideSelector } from '@/components/trade/LimitOrderBidAskSideSelector'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { LimitOrderExpiryField } from '@/components/trade/LimitOrderExpiryField'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateLimitOrderNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { describeLimitCrossingBlocker } from '@/utils/limitOrderNonCrossing'
import { formatLimitBatchGasSavingsLine, formatLimitLadderPlacementSummary } from '@/utils/limitOrderBatchGasSummary'
import { buildLadderSpecWire, ladderRungsToBatchItems } from '@/utils/limitLadderPlacementPlan'
import {
  expandLimitLadder,
  LimitLadderError,
  sumLadderAmountsRaw,
  type LimitLadderSpec,
} from '@/utils/limitOrderLadder'
import { LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT } from '@/utils/limitOrderExpiry'
import { toRawAmount } from '@/utils/formatAmount'
import { warnIndexerPlacementPollFailed } from '@/utils/warnIndexerPlacementPollFailed'

export interface LimitOrderLadderPanelProps {
  pairAddress: string
  walletAddress: string
  escrowToken: string
  escrowDecimals: number
  token0Symbol: string
  token1Symbol: string
  disabled?: boolean
  onPlaced?: (orderIds: number[]) => void
}

export function LimitOrderLadderPanel({
  pairAddress,
  walletAddress,
  escrowToken,
  escrowDecimals,
  token0Symbol,
  token1Symbol,
  disabled,
  onPlaced,
}: LimitOrderLadderPanelProps) {
  const queryClient = useQueryClient()
  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  const [startPrice, setStartPrice] = useState('0.95')
  const [endPrice, setEndPrice] = useState('1.05')
  const [rungCount, setRungCount] = useState(5)
  const [rungCountInput, setRungCountInput] = useState('5')
  const [totalHuman, setTotalHuman] = useState('100')
  const { maxSteps, setMaxSteps, expiresAt, setExpiresAt, limitAdvancedOpen, setLimitAdvancedOpen } =
    useLimitOrderForm()
  const maxStepsTouchedRef = useRef(false)

  const configQuery = useLimitOrderConfig(pairAddress)
  const maxRungs = configQuery.data?.max_batch_rungs ?? 20

  const handleRungCountChange = (raw: string) => {
    setRungCountInput(raw)
    if (raw.trim() === '') return
    const parsed = Number(raw)
    if (Number.isInteger(parsed) && parsed >= 2 && parsed <= maxRungs) {
      setRungCount(parsed)
    }
  }

  const handleRungCountBlur = () => {
    const parsed = Number(rungCountInput)
    if (rungCountInput.trim() === '' || !Number.isInteger(parsed)) {
      setRungCountInput(String(rungCount))
      return
    }
    const clamped = Math.min(maxRungs, Math.max(2, parsed))
    setRungCount(clamped)
    setRungCountInput(String(clamped))
  }

  const rungCountError =
    rungCountInput.trim() !== '' &&
    (() => {
      const parsed = Number(rungCountInput)
      if (!Number.isInteger(parsed)) return 'Rung count must be a whole number'
      if (parsed < 2) return 'Rung count must be at least 2'
      if (parsed > maxRungs) return `Rung count can be at most ${maxRungs} on this pair`
      return null
    })()

  const preview = useMemo(() => {
    try {
      const totalRaw = toRawAmount(totalHuman, escrowDecimals)
      const spec: LimitLadderSpec = {
        side,
        startPrice,
        endPrice,
        count: rungCount,
        totalAmountRaw: totalRaw,
        distribution: 'equal',
        maxAdjustSteps: maxSteps,
        expiresAt: expiresAt ?? null,
      }
      return { rungs: expandLimitLadder(spec, maxRungs), error: null as string | null }
    } catch (e) {
      const msg = e instanceof LimitLadderError ? e.message : (e as Error).message
      return { rungs: [] as ReturnType<typeof expandLimitLadder>, error: msg }
    }
  }, [side, startPrice, endPrice, rungCount, totalHuman, escrowDecimals, maxSteps, expiresAt, maxRungs])

  const placementPlanQuery = useLimitLadderPlacementPlan({
    pairAddress,
    side,
    startPrice,
    endPrice,
    count: rungCount,
    rungs: preview.rungs,
    maxAdjustSteps: maxSteps,
    expiresAt,
    enabled: !preview.error && preview.rungs.length >= 2,
  })

  useEffect(() => {
    const recommended = placementPlanQuery.data?.recommendedMaxSteps
    if (recommended == null || maxStepsTouchedRef.current) return
    if (maxSteps === LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT) {
      setMaxSteps(recommended)
    }
  }, [placementPlanQuery.data?.recommendedMaxSteps, maxSteps, setMaxSteps])

  const placeGates = useLimitLadderPlaceGates(walletAddress, escrowToken, totalHuman, escrowDecimals, rungCount)

  const { bestBid, bestAsk } = useTradeBestBookPrices(pairAddress)

  const ladderCrossingGate = useMemo(() => {
    if (preview.error || preview.rungs.length === 0) {
      return { canPlaceLimit: true, userMessage: null, tone: 'none' as const }
    }
    let crossingCount = 0
    let firstReason: string | null = null
    for (const r of preview.rungs) {
      const reason = describeLimitCrossingBlocker(side, r.price, bestBid, bestAsk)
      if (reason) {
        crossingCount += 1
        if (firstReason == null) firstReason = reason
      }
    }
    if (crossingCount === 0) {
      return { canPlaceLimit: true, userMessage: null, tone: 'none' as const }
    }
    return {
      canPlaceLimit: false,
      userMessage: `${crossingCount} of ${preview.rungs.length} rungs will cross the market and execute immediately as taker orders. ${firstReason}`,
      tone: 'warning' as const,
    }
  }, [preview.error, preview.rungs, side, bestBid, bestAsk])

  const placeMutation = useMutation({
    mutationFn: async () => {
      if (preview.error || preview.rungs.length < 2) {
        throw new Error(preview.error ?? 'Invalid ladder')
      }
      const escrowGate = evaluateLimitOrderEscrowPlaceGate(totalHuman, escrowDecimals, placeGates.escrowBalanceQuery)
      if (!escrowGate.canPlaceLimit) {
        throw new Error(escrowGate.userMessage ?? 'Insufficient balance')
      }
      const nativeGate = evaluateLimitOrderNativeGasPlaceGate(
        totalHuman,
        escrowDecimals,
        placeGates.nativeUlunaQuery,
        placeGates.batchMinUluna
      )
      if (!nativeGate.canPlaceLimit) {
        throw new Error(nativeGate.userMessage ?? 'Insufficient LUNC for gas')
      }
      for (const r of preview.rungs) {
        const cross = describeLimitCrossingBlocker(side, r.price, bestBid, bestAsk)
        if (cross) {
          throw new Error(cross)
        }
      }

      const plan = placementPlanQuery.data
      const totalRaw = sumLadderAmountsRaw(preview.rungs)
      const exp = expiresAt ?? undefined

      if (plan?.path === 'deep_batch') {
        const orders = ladderRungsToBatchItems(preview.rungs, plan.hints, maxSteps, exp)
        return placeLimitOrderBatchWithAllowance(walletAddress, escrowToken, pairAddress, totalRaw, side, orders)
      }

      const ladderSpec = buildLadderSpecWire({
        side,
        startPrice,
        endPrice,
        count: rungCount,
        totalAmountRaw: totalRaw,
        maxAdjustSteps: maxSteps,
        expiresAt: exp,
        plan: plan ?? {
          path: 'thin_ladder',
          recommendedMaxSteps: maxSteps,
          skipRisk: {
            score: 0,
            predictedPlaced: rungCount,
            predictedSkipped: 0,
            needsHintedBatchPath: false,
          },
          depth: {
            windowOrderCount: 0,
            foreignOrdersBetweenRungs: 0,
            headToBoundaryDistance: 0,
            unresolvedHintCount: 0,
          },
          hints: [],
          probeDegraded: true,
          notes: [],
        },
      })

      return placeLimitOrderLadderWithAllowance(walletAddress, escrowToken, pairAddress, totalRaw, ladderSpec)
    },
    onSuccess: async (txHash) => {
      void queryClient.invalidateQueries({ queryKey: ['limitPlacements', pairAddress] })
      void queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddress] })
      void queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddress] })
      void queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddress] })
      const before = await getPairLimitPlacements(pairAddress, { limit: 100 })
      const known = new Set(before.map((p) => p.order_id))
      const found: number[] = []
      for (let i = 0; i < 24; i++) {
        await new Promise((r) => setTimeout(r, 500))
        const next = await getPairLimitPlacements(pairAddress, { limit: 100 })
        for (const p of next) {
          if (!known.has(p.order_id)) found.push(p.order_id)
        }
        if (found.length >= rungCount) break
      }
      if (found.length < rungCount) {
        warnIndexerPlacementPollFailed({ expected: rungCount, found: found.length, txHash })
      }
      onPlaced?.(found)
    },
  })

  const gasSummaryLine = formatLimitBatchGasSavingsLine(
    rungCount,
    placeGates.batchMinUluna,
    placeGates.gasSavingsUlunaVsSeparate
  )
  const placementSummaryLine = formatLimitLadderPlacementSummary(rungCount, maxSteps, placementPlanQuery.data)
  const planNotes = placementPlanQuery.data?.notes ?? []

  const submitDisabled =
    disabled ||
    placeMutation.isPending ||
    Boolean(preview.error) ||
    !placeGates.canPlace ||
    !ladderCrossingGate.canPlaceLimit

  return (
    <div className="space-y-3" data-testid="limit-order-ladder-panel">
      <LimitOrderBidAskSideSelector
        idPrefix="ladder"
        side={side}
        onSideChange={setSide}
        bidLabel={`Bid (${token1Symbol})`}
        askLabel={`Ask (${token0Symbol})`}
      />
      <div className="grid grid-cols-2 gap-2">
        <label className="block text-sm">
          <span className="text-muted">Start price</span>
          <input
            className="input-neo mt-1 w-full"
            value={startPrice}
            onChange={(e) => setStartPrice(e.target.value)}
            data-testid="ladder-start-price"
          />
        </label>
        <label className="block text-sm">
          <span className="text-muted">End price</span>
          <input
            className="input-neo mt-1 w-full"
            value={endPrice}
            onChange={(e) => setEndPrice(e.target.value)}
            data-testid="ladder-end-price"
          />
        </label>
      </div>
      <label className="block text-sm">
        <span className="text-muted">Rung count (max {maxRungs} on this pair)</span>
        <input
          type="number"
          min={2}
          max={maxRungs}
          className="input-neo mt-1 w-full"
          value={rungCountInput}
          onChange={(e) => handleRungCountChange(e.target.value)}
          onBlur={handleRungCountBlur}
          data-testid="ladder-rung-count"
        />
        {rungCountError && (
          <p className="mt-1 text-sm text-red-400" data-testid="ladder-rung-count-error">
            {rungCountError}
          </p>
        )}
      </label>
      <label className="block text-sm">
        <span className="text-muted">Total escrow ({side === 'bid' ? token1Symbol : token0Symbol})</span>
        <input
          className="input-neo mt-1 w-full"
          value={totalHuman}
          onChange={(e) => setTotalHuman(e.target.value)}
          data-testid="ladder-total-amount"
        />
      </label>
      <LimitOrderExpiryField value={expiresAt} onChange={setExpiresAt} idPrefix="ladder" />
      <LimitOrderAdvancedLimitSettings
        open={limitAdvancedOpen}
        onOpenChange={setLimitAdvancedOpen}
        maxSteps={maxSteps}
        onMaxStepsChange={(v) => {
          maxStepsTouchedRef.current = true
          setMaxSteps(v)
        }}
        expiresAt={expiresAt}
        onExpiresAtChange={setExpiresAt}
        idPrefix="ladder"
      />
      {preview.error && <p className="text-sm text-red-400">{preview.error}</p>}
      {!preview.error && preview.rungs.length > 0 && (
        <div className="text-xs text-muted overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr>
                <th className="text-left">#</th>
                <th className="text-left">Price</th>
                <th className="text-right">Escrow (raw)</th>
              </tr>
            </thead>
            <tbody>
              {preview.rungs.map((r, i) => (
                <tr key={i}>
                  <td>{i + 1}</td>
                  <td>{r.price}</td>
                  <td className="text-right font-mono">{r.amountRaw}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="mt-2" data-testid="ladder-gas-summary">
            {gasSummaryLine}
          </p>
          <p className="mt-1" data-testid="ladder-placement-summary">
            {placementPlanQuery.isLoading ? 'Probing book depth…' : placementSummaryLine}
          </p>
          {planNotes.map((note, i) => (
            <p key={i} className="mt-1 text-amber-400/90" data-testid="ladder-placement-note">
              {note}
            </p>
          ))}
        </div>
      )}
      <LimitOrderEscrowPlaceGuardMessage gate={placeGates.inlineGate} data-testid="ladder-place-guard" />
      <LimitOrderEscrowPlaceGuardMessage gate={ladderCrossingGate} data-testid="ladder-crossing-guard" />
      <button
        type="button"
        className="btn-primary btn-cta w-full !text-xs"
        disabled={submitDisabled}
        data-testid="ladder-place-submit"
        onClick={() => placeMutation.mutate()}
      >
        {placeMutation.isPending ? 'Placing ladder…' : `Place ${rungCount}-rung ladder`}
      </button>
      {placeMutation.isError && <TxResultAlert type="error" message={(placeMutation.error as Error).message} />}
      {placeMutation.isSuccess && (
        <TxResultAlert type="success" message="Ladder submitted." txHash={placeMutation.data} />
      )}
    </div>
  )
}
