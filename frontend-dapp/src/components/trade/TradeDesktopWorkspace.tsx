import type { ReactNode } from 'react'
import { sounds } from '@/lib/sounds'
import {
  tradeDesktopChartGridColumn,
  tradeDesktopGridTemplateColumns,
  tradeDesktopTicketGridColumn,
} from '@/utils/tradeWorkspacePanels'

type TradeDesktopWorkspaceProps = {
  bookVisible: boolean
  ticketVisible: boolean
  tapeExpanded: boolean
  onBookVisibleChange: (visible: boolean) => void
  onTicketVisibleChange: (visible: boolean) => void
  onTapeExpandedChange: (expanded: boolean) => void
  book: ReactNode
  chart: ReactNode
  ticket: ReactNode
  tape: ReactNode
}

function PanelToggle({
  testId,
  pressed,
  label,
  onPressedChange,
}: {
  testId: string
  pressed: boolean
  label: string
  onPressedChange: (next: boolean) => void
}) {
  return (
    <button
      type="button"
      className="btn-muted !text-[11px] !px-2.5 !py-1"
      data-testid={testId}
      aria-pressed={pressed}
      onClick={() => {
        sounds.playButtonPress()
        onPressedChange(!pressed)
      }}
    >
      {label}
    </button>
  )
}

/**
 * Desktop `/trade` workspace: CSS grid (book | chart | ticket) + independent tape row.
 * No drag-resize. Hidden side panels stay mounted (`hidden` + `inert`) so the ticket is not remounted (GitLab #561 / #178).
 */
export function TradeDesktopWorkspace({
  bookVisible,
  ticketVisible,
  tapeExpanded,
  onBookVisibleChange,
  onTicketVisibleChange,
  onTapeExpandedChange,
  book,
  chart,
  ticket,
  tape,
}: TradeDesktopWorkspaceProps) {
  const chartCol = tradeDesktopChartGridColumn(bookVisible)
  const ticketCol = tradeDesktopTicketGridColumn(bookVisible)

  return (
    <div className="trade-desktop-workspace flex" data-testid="trade-desktop-workspace">
      <div className="flex flex-wrap items-center gap-2 shrink-0 px-0.5" data-testid="trade-desktop-panel-toggles">
        <PanelToggle
          testId="trade-desktop-book-toggle"
          pressed={bookVisible}
          label="Order book"
          onPressedChange={onBookVisibleChange}
        />
        <PanelToggle
          testId="trade-desktop-ticket-toggle"
          pressed={ticketVisible}
          label="Ticket"
          onPressedChange={onTicketVisibleChange}
        />
      </div>

      <div
        className="trade-desktop-grid"
        data-testid="trade-desktop-grid"
        style={{ gridTemplateColumns: tradeDesktopGridTemplateColumns(bookVisible, ticketVisible) }}
      >
        <div
          className={bookVisible ? 'min-h-0 min-w-0 overflow-hidden' : 'hidden'}
          style={bookVisible ? { gridColumn: 1, gridRow: 1 } : undefined}
          data-testid="trade-desktop-book-col"
          {...(!bookVisible ? { inert: true, 'aria-hidden': true } : {})}
        >
          {book}
        </div>

        <div
          className="min-h-0 min-w-0 flex flex-col h-full"
          style={{ gridColumn: chartCol, gridRow: 1 }}
          data-testid="trade-desktop-chart-col"
        >
          {chart}
        </div>

        <div
          className={ticketVisible ? 'min-h-0 min-w-0 flex flex-col overflow-hidden' : 'hidden'}
          style={ticketVisible ? { gridColumn: ticketCol, gridRow: 1 } : undefined}
          data-testid="trade-desktop-ticket-col"
          {...(!ticketVisible ? { inert: true, 'aria-hidden': true } : {})}
        >
          {ticket}
        </div>

        <div
          className="min-h-0 min-w-0 card-glass !p-3 flex flex-col"
          style={{ gridColumn: '1 / -1', gridRow: 2 }}
          data-testid="trade-desktop-tape-panel"
          data-expanded={tapeExpanded ? 'true' : 'false'}
        >
          <div className="flex items-center justify-between gap-2 mb-2 shrink-0">
            <h2 className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
              Recent trades
            </h2>
            <button
              type="button"
              className="btn-muted !text-[10px] !px-2 !py-1"
              data-testid="trade-desktop-tape-toggle"
              aria-expanded={tapeExpanded}
              onClick={() => {
                sounds.playButtonPress()
                onTapeExpandedChange(!tapeExpanded)
              }}
            >
              {tapeExpanded ? 'Collapse' : 'Expand'}
            </button>
          </div>
          {tapeExpanded ? (
            <div className="trade-desktop-tape-body flex-1 min-h-0 overflow-y-auto">{tape}</div>
          ) : (
            <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-subtle)' }}>
              Expand for live trades.
            </p>
          )}
        </div>
      </div>
    </div>
  )
}
