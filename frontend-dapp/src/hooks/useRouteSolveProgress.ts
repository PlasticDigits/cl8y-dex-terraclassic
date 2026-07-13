import { useEffect, useRef, useState } from 'react'
import { getRouteSolveProgress } from '@/services/indexer/client'
import { SIM_QUOTE_PROGRESS_POLL_MS, type RouteSolveProgressSnapshot } from '@/utils/routeSolveProgress'

export type UseRouteSolveProgressArgs = {
  /** When false, clears progress and stops polling. */
  enabled: boolean
  isFetching: boolean
  tokenIn?: string
  tokenOut?: string
  amountIn?: string
  trader?: string
  maxMakerFills?: number
}

/**
 * Polls `GET /api/v1/route/solve/progress` ~1 Hz while a sim quote is in flight (GitLab #485).
 * Display-only — does not gate submit or receive amount (#484).
 */
export function useRouteSolveProgress({
  enabled,
  isFetching,
  tokenIn,
  tokenOut,
  amountIn,
  trader,
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

    const poll = async () => {
      try {
        const next = await getRouteSolveProgress(tin, tout, amt, {
          trader,
          maxMakerFills,
          signal: ac.signal,
        })
        if (!cancelled && !ac.signal.aborted) {
          setProgress(next)
        }
      } catch {
        // Advisory only — ignore poll failures (outage / abort / 400).
      }
    }

    void poll()
    const pollId = window.setInterval(() => {
      void poll()
    }, SIM_QUOTE_PROGRESS_POLL_MS)
    const tickId = window.setInterval(() => setNowMs(Date.now()), 250)

    return () => {
      cancelled = true
      window.clearInterval(pollId)
      window.clearInterval(tickId)
      ac.abort()
    }
  }, [enabled, isFetching, tokenIn, tokenOut, amountIn, trader, maxMakerFills])

  return { progress, fetchStartedAtMs, nowMs }
}
