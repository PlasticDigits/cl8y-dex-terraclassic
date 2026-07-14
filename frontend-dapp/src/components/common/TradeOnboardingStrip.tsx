import { useState } from 'react'
import { Link } from 'react-router-dom'
import { sounds } from '@/lib/sounds'
import { readTradeOnboardingDismissed, writeTradeOnboardingDismissed } from '@/utils/tradeOnboarding'

/**
 * First-visit IA strip: Swap vs Trade vs Limits (GitLab #417).
 * Dismiss persists in localStorage; does not block wallet or submit actions.
 */
export function TradeOnboardingStrip() {
  const [visible, setVisible] = useState(() => !readTradeOnboardingDismissed())

  if (!visible) return null

  const dismiss = () => {
    sounds.playButtonPress()
    writeTradeOnboardingDismissed(true)
    setVisible(false)
  }

  return (
    <aside
      className="rounded-2xl border border-white/10 px-3 py-3 sm:px-4 sm:py-3.5 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between sm:gap-4"
      style={{ background: 'rgba(255, 255, 255, 0.03)' }}
      data-testid="trade-onboarding-strip"
      aria-label="Getting started"
    >
      <div className="min-w-0 space-y-1 text-sm" style={{ color: 'var(--ink-dim)' }}>
        <p className="font-semibold" style={{ color: 'var(--ink)' }}>
          New?{' '}
          <Link to="/" className="underline hover:opacity-80" data-testid="trade-onboarding-swap-link">
            Start with Swap
          </Link>
        </p>
        <p className="leading-snug text-xs">
          <strong style={{ color: 'var(--ink)' }}>Swap</strong> · <strong style={{ color: 'var(--ink)' }}>Trade</strong>{' '}
          · <strong style={{ color: 'var(--ink)' }}>Limits</strong>
        </p>
      </div>
      <button
        type="button"
        className="btn-muted shrink-0 self-start !px-3 !py-2 !text-xs"
        onClick={dismiss}
        data-testid="trade-onboarding-dismiss"
        aria-label="Dismiss getting started tips"
      >
        Dismiss
      </button>
    </aside>
  )
}
