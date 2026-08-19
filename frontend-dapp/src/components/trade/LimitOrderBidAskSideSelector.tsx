import { useCallback, useId, useRef } from 'react'
import type { KeyboardEvent } from 'react'
import { limitSideControlClass } from '@/components/trade/limitSideControlClass'

export type LimitOrderSide = 'bid' | 'ask'

export type LimitOrderBidAskSideSelectorProps = {
  side: LimitOrderSide
  onSideChange: (next: LimitOrderSide) => void
  bidLabel: string
  askLabel: string
  /** Used for `data-testid` and accessible naming. */
  idPrefix: string
  compact?: boolean
}

/**
 * Bid / Ask control for limit place: button-based radiogroup (WAI-ARIA) for immediate
 * selection feedback and larger targets vs native radios (GitLab #153).
 * Buy/Sell fills follow bid=Buy / ask=Sell — not displayed-token color (GitLab #563).
 */
export function LimitOrderBidAskSideSelector({
  side,
  onSideChange,
  bidLabel,
  askLabel,
  idPrefix,
  compact,
}: LimitOrderBidAskSideSelectorProps) {
  const legendId = useId()
  const bidRef = useRef<HTMLButtonElement>(null)
  const askRef = useRef<HTMLButtonElement>(null)

  const focusBid = useCallback(() => {
    bidRef.current?.focus()
  }, [])

  const focusAsk = useCallback(() => {
    askRef.current?.focus()
  }, [])

  const onBidKeyDown = useCallback(
    (e: KeyboardEvent<HTMLButtonElement>) => {
      if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
        e.preventDefault()
        onSideChange('ask')
        queueMicrotask(focusAsk)
      } else if (e.key === 'End') {
        e.preventDefault()
        onSideChange('ask')
        queueMicrotask(focusAsk)
      }
    },
    [focusAsk, onSideChange]
  )

  const onAskKeyDown = useCallback(
    (e: KeyboardEvent<HTMLButtonElement>) => {
      if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
        e.preventDefault()
        onSideChange('bid')
        queueMicrotask(focusBid)
      } else if (e.key === 'Home') {
        e.preventDefault()
        onSideChange('bid')
        queueMicrotask(focusBid)
      }
    },
    [focusBid, onSideChange]
  )

  return (
    <div
      role="radiogroup"
      aria-labelledby={legendId}
      className={compact ? 'grid grid-cols-2 gap-2' : 'flex flex-wrap gap-2'}
      data-testid={`${idPrefix}-side-radiogroup`}
    >
      <span id={legendId} className="sr-only">
        Limit order side — bid or ask
      </span>
      <button
        ref={bidRef}
        type="button"
        role="radio"
        aria-checked={side === 'bid'}
        tabIndex={side === 'bid' ? 0 : -1}
        data-testid={`${idPrefix}-side-bid`}
        className={limitSideControlClass({ compact, tone: 'buy', selected: side === 'bid' })}
        onClick={() => onSideChange('bid')}
        onKeyDown={onBidKeyDown}
      >
        {bidLabel}
      </button>
      <button
        ref={askRef}
        type="button"
        role="radio"
        aria-checked={side === 'ask'}
        tabIndex={side === 'ask' ? 0 : -1}
        data-testid={`${idPrefix}-side-ask`}
        className={limitSideControlClass({ compact, tone: 'sell', selected: side === 'ask' })}
        onClick={() => onSideChange('ask')}
        onKeyDown={onAskKeyDown}
      >
        {askLabel}
      </button>
    </div>
  )
}
