import { describe, it, expect } from 'vitest'
import {
  formatRouteSolveSearchProgress,
  nextProgressPollDelayMs,
  progressPollTraderParams,
  resolveSimQuoteLoadingLabel,
  shouldShowRouteSolveProgress,
  shouldStopProgressPolling,
  SIM_QUOTE_PROGRESS_GIVE_UP_AFTER,
  SIM_QUOTE_PROGRESS_MAX_BACKOFF_MS,
  SIM_QUOTE_PROGRESS_MIN_VISIBLE_MS,
  SIM_QUOTE_PROGRESS_POLL_MS,
} from './routeSolveProgress'

describe('formatRouteSolveSearchProgress (#485)', () => {
  it('prefers indexer label', () => {
    expect(formatRouteSolveSearchProgress({ done: 2, total: 5, label: 'Searching 2 of 5 paths…' })).toBe(
      'Searching 2 of 5 paths…'
    )
  })

  it('falls back to x of y when label empty', () => {
    expect(formatRouteSolveSearchProgress({ done: 3, total: 12, label: '  ' })).toBe('Searching 3 of 12 paths…')
  })
})

describe('shouldShowRouteSolveProgress (#485)', () => {
  const progress = { stage: 'evaluating', done: 1, total: 5, label: 'Searching 1 of 5 paths…' }

  it('hides until min visible elapsed', () => {
    expect(shouldShowRouteSolveProgress(1000, progress, 1000 + SIM_QUOTE_PROGRESS_MIN_VISIBLE_MS - 1)).toBe(false)
    expect(shouldShowRouteSolveProgress(1000, progress, 1000 + SIM_QUOTE_PROGRESS_MIN_VISIBLE_MS)).toBe(true)
  })

  it('hides idle / terminal / missing', () => {
    expect(shouldShowRouteSolveProgress(0, { ...progress, stage: 'idle' }, 10_000)).toBe(false)
    expect(shouldShowRouteSolveProgress(0, { ...progress, stage: 'done' }, 10_000)).toBe(false)
    expect(shouldShowRouteSolveProgress(null, progress, 10_000)).toBe(false)
  })
})

describe('resolveSimQuoteLoadingLabel (#485)', () => {
  const progress = { stage: 'evaluating', done: 2, total: 5, label: 'Searching 2 of 5 paths…' }

  it('uses progress label after delay when no settled quote', () => {
    expect(resolveSimQuoteLoadingLabel(true, false, progress, 0, SIM_QUOTE_PROGRESS_MIN_VISIBLE_MS)).toBe(
      'Searching 2 of 5 paths…'
    )
  })

  it('keeps Calculating fallback when settled quote exists (background refetch)', () => {
    expect(resolveSimQuoteLoadingLabel(true, true, progress, 0, SIM_QUOTE_PROGRESS_MIN_VISIBLE_MS)).toBe(
      'Calculating...'
    )
  })

  it('uses Quoting… fallback when requested', () => {
    expect(resolveSimQuoteLoadingLabel(true, false, null, 0, 10_000, 'Quoting…')).toBe('Quoting…')
  })
})

describe('progress poll backoff (#694)', () => {
  it('starts at 1 Hz and doubles up to 8s', () => {
    expect(nextProgressPollDelayMs(0)).toBe(SIM_QUOTE_PROGRESS_POLL_MS)
    expect(nextProgressPollDelayMs(1)).toBe(2_000)
    expect(nextProgressPollDelayMs(2)).toBe(4_000)
    expect(nextProgressPollDelayMs(3)).toBe(SIM_QUOTE_PROGRESS_MAX_BACKOFF_MS)
    expect(nextProgressPollDelayMs(10)).toBe(SIM_QUOTE_PROGRESS_MAX_BACKOFF_MS)
  })

  it('stops after consecutive failures', () => {
    expect(shouldStopProgressPolling(SIM_QUOTE_PROGRESS_GIVE_UP_AFTER - 1)).toBe(false)
    expect(shouldStopProgressPolling(SIM_QUOTE_PROGRESS_GIVE_UP_AFTER)).toBe(true)
  })
})

describe('progressPollTraderParams (#694)', () => {
  it('omits trader when discount is known', () => {
    expect(progressPollTraderParams({ knownDiscountBps: 5000, trader: 'terra1abc' })).toEqual({
      discountBps: 5000,
    })
    expect(progressPollTraderParams({ knownDiscountBps: 0, trader: 'terra1abc' })).toEqual({
      discountBps: 0,
    })
  })

  it('keeps trader when discount is not yet known', () => {
    expect(progressPollTraderParams({ trader: 'terra1abc' })).toEqual({ trader: 'terra1abc' })
    expect(progressPollTraderParams({})).toEqual({})
  })
})
