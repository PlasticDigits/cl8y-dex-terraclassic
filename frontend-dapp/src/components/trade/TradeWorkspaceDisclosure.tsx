import { useId, useState, type ReactNode } from 'react'
import { sounds } from '@/lib/sounds'
import { readTradePanelExpanded, writeTradePanelExpanded } from '@/utils/tradeWorkspacePanels'

type TradeWorkspaceDisclosureProps = {
  title: string
  storageKey: string
  defaultExpanded?: boolean
  testId: string
  children: ReactNode
  className?: string
}

/**
 * Collapsible secondary trade workspace block with persisted open state (GitLab #417).
 */
export function TradeWorkspaceDisclosure({
  title,
  storageKey,
  defaultExpanded = false,
  testId,
  children,
  className = '',
}: TradeWorkspaceDisclosureProps) {
  const contentId = useId()
  const [expanded, setExpanded] = useState(() => readTradePanelExpanded(storageKey, defaultExpanded))

  const toggle = () => {
    sounds.playButtonPress()
    setExpanded((open) => {
      const next = !open
      writeTradePanelExpanded(storageKey, next)
      return next
    })
  }

  return (
    <section className={className} data-testid={testId}>
      <button
        type="button"
        className="flex w-full items-center justify-between gap-2 rounded-xl border border-white/10 px-3 py-2.5 text-left"
        style={{ background: 'rgba(255, 255, 255, 0.025)' }}
        aria-expanded={expanded}
        aria-controls={contentId}
        data-testid={`${testId}-toggle`}
        onClick={toggle}
      >
        <span className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
          {title}
        </span>
        <span className="text-xs" style={{ color: 'var(--ink-subtle)' }} aria-hidden="true">
          {expanded ? '▲' : '▼'}
        </span>
      </button>
      {expanded ? (
        <div id={contentId} className="mt-2" data-testid={`${testId}-content`}>
          {children}
        </div>
      ) : null}
    </section>
  )
}
