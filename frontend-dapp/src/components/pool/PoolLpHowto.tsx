import { useEffect, useRef, useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { sounds } from '@/lib/sounds'
import { readPoolLpHowtoHintDismissed, writePoolLpHowtoHintDismissed } from '@/utils/poolLpHowto'
import {
  POOL_LP_HOWTO_ANCHOR,
  POOL_LP_HOWTO_DISMISS_LABEL,
  POOL_LP_HOWTO_HINT,
  POOL_LP_HOWTO_LINKS,
  POOL_LP_HOWTO_OPEN_LABEL,
  POOL_LP_HOWTO_STEPS,
  POOL_LP_HOWTO_SUMMARY,
} from '@/utils/poolLpHowtoCopy'

function hashTargetsHowto(hash: string): boolean {
  return hash.replace(/^#/, '') === POOL_LP_HOWTO_ANCHOR
}

/**
 * Opt-in retail how-to on /pool (GitLab #531).
 * In-flow only — must not use position:fixed or cover Provide / wallet / clickwrap.
 */
export function PoolLpHowto() {
  const location = useLocation()
  const detailsRef = useRef<HTMLDetailsElement>(null)
  const [hintVisible, setHintVisible] = useState(() => !readPoolLpHowtoHintDismissed())

  const openDetails = () => {
    const el = detailsRef.current
    if (!el) return
    el.open = true
  }

  useEffect(() => {
    if (hashTargetsHowto(location.hash)) openDetails()
  }, [location.hash])

  const dismissHint = () => {
    sounds.playButtonPress()
    writePoolLpHowtoHintDismissed(true)
    setHintVisible(false)
  }

  const openFromHint = () => {
    sounds.playButtonPress()
    openDetails()
    detailsRef.current?.scrollIntoView?.({ block: 'nearest' })
  }

  return (
    <section
      id={POOL_LP_HOWTO_ANCHOR}
      className="relative mb-4 space-y-2"
      data-testid="pool-lp-howto"
      aria-label={POOL_LP_HOWTO_SUMMARY}
    >
      {hintVisible ? (
        <div
          className="rounded-2xl border border-white/10 px-3 py-3 sm:px-4 sm:py-3.5 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between sm:gap-4"
          style={{ background: 'rgba(255, 255, 255, 0.03)' }}
          data-testid="pool-lp-howto-hint"
        >
          <p className="min-w-0 text-sm" style={{ color: 'var(--ink-dim)' }}>
            {POOL_LP_HOWTO_HINT}
          </p>
          <div className="flex shrink-0 gap-2 self-start">
            <button
              type="button"
              className="btn-muted !px-3 !py-2 !text-xs"
              onClick={openFromHint}
              data-testid="pool-lp-howto-open"
            >
              {POOL_LP_HOWTO_OPEN_LABEL}
            </button>
            <button
              type="button"
              className="btn-muted !px-3 !py-2 !text-xs"
              onClick={dismissHint}
              data-testid="pool-lp-howto-dismiss"
              aria-label="Dismiss liquidity how-to hint"
            >
              {POOL_LP_HOWTO_DISMISS_LABEL}
            </button>
          </div>
        </div>
      ) : null}

      <details ref={detailsRef} className="card-glass !p-4" data-testid="pool-lp-howto-details">
        <summary
          className="text-sm font-semibold uppercase tracking-wide cursor-pointer"
          data-testid="pool-lp-howto-summary"
        >
          {POOL_LP_HOWTO_SUMMARY}
        </summary>
        <ol className="mt-3 space-y-2 text-xs leading-relaxed list-decimal pl-4" style={{ color: 'var(--ink-dim)' }}>
          {POOL_LP_HOWTO_STEPS.map((step) => (
            <li key={step.id} data-testid={`pool-lp-howto-step-${step.id}`}>
              {step.text}
            </li>
          ))}
        </ol>
        <p className="mt-3 flex flex-wrap gap-x-3 gap-y-1 text-xs" style={{ color: 'var(--ink-subtle)' }}>
          {POOL_LP_HOWTO_LINKS.map((link) => (
            <Link
              key={link.href}
              to={link.href}
              className="underline hover:opacity-80"
              data-testid={link.testId}
              onClick={() => sounds.playButtonPress()}
            >
              {link.label}
            </Link>
          ))}
        </p>
      </details>
    </section>
  )
}
