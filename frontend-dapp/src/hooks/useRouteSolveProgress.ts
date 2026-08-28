import { useEffect, useRef, useState } from 'react'
import { getRouteSolveProgress } from '@/services/indexer/client'
import {
  nextProgressPollDelayMs,
  progressPollTraderParams,
  shouldStopProgressPolling,
  type RouteSolveProgressSnapshot,
} from '@/utils/routeSolveProgress'

export type UseRouteSolveProgressArgs = {
  /** When false, clears progress and stops polling. */
  enabled: boolean
  isFetching: boolean
  tokenIn?: string
  tokenOut?: string
  amountIn?: string
  trader?: string
  /** When set (including 0), omit `trader` and pass `discount_bps` (#694). */
  knownDiscountBps?: number
  maxMakerFills?: number
}

/**
 * Polls `GET /api/v1/route/solve/progress` ~1 Hz while a sim quote is in flight (GitLab #485).
 * Display-only — does not gate submit or receive amount (#484).
 * Backs off after consecutive failures; omits `trader` when discount is already known (#694).
 */
export function useRouteSolveProgress({
  enabled,
  isFetching,
  tokenIn,
  tokenOut,
  amountIn,
  trader,
  knownDiscountBps,
  maxMakerFills,
}: UseRouteSolveProgressArgs): {
  progress: RouteSolveProgressSnapshot | null
  fetchStartedAtMs: number | null
  nowMs: number
} {
  const [progress, setProgress] = useState<RouteSolveProgressSnapshot | null>(null)
  const [fetchStartedAtMs, setFetchStartedAtMs] = useState<number | null>(null)
  const [nowMs, setNowMs] = useState(() => Date.now())
  const abortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    if (!enabled || !isFetching) {
      abortRef.current?.abort()
      abortRef.current = null
      setProgress(null)
      setFetchStartedAtMs(null)
      return
    }

    const tin = tokenIn?.trim()
    const tout = tokenOut?.trim()
    const amt = amountIn?.trim()
    if (!tin || !tout || !amt || amt === '0') {
      setProgress(null)
      setFetchStartedAtMs(null)
      return
    }

    setFetchStartedAtMs((prev) => prev ?? Date.now())
    const ac = new AbortController()
    abortRef.current?.abort()
    abortRef.current = ac

    let cancelled = false
    let consecutiveFailures = 0
    let pollTimer: number | undefined

    const traderParams = progressPollTraderParams({ knownDiscountBps, trader })

    const poll = async () => {
      try {
        const next = await getRouteSolveProgress(tin, tout, amt, {
          ...traderParams,
          maxMakerFills,
          signal: ac.signal,
        })
        if (!cancelled && !ac.signal.aborted) {
          consecutiveFailures = 0
          setProgress(next)
        }
      } catch (err) {
        if (ac.signal.aborted || cancelled) return
        const name = err instanceof Error ? err.name : ''
        if (name === 'AbortError') return
        consecutiveFailures += 1
      }
    }

    const schedule = () => {
      if (cancelled || shouldStopProgressPolling(consecutiveFailures)) return
      pollTimer = window.setTimeout(() => {
        void (async () => {
          await poll()
          schedule()
        })()
      }, nextProgressPollDelayMs(consecutiveFailures))
    }

    void (async () => {
      await poll()
      schedule()
    })()
    const tickId = window.setInterval(() => setNowMs(Date.now()), 250)

    return () => {
      cancelled = true
      if (pollTimer != null) window.clearTimeout(pollTimer)
      window.clearInterval(tickId)
      ac.abort()
    }
  }, [enabled, isFetching, tokenIn, tokenOut, amountIn, trader, knownDiscountBps, maxMakerFills])

  return { progress, fetchStartedAtMs, nowMs }
}
