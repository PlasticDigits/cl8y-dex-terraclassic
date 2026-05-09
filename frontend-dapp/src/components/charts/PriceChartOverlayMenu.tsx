import { useEffect, useId, useRef, useState } from 'react'
import { sounds } from '@/lib/sounds'

export interface PriceChartOverlayMenuProps {
  showSma7: boolean
  showSma25: boolean
  showRsi: boolean
  onToggleSma7: () => void
  onToggleSma25: () => void
  onToggleRsi: () => void
}

/** Indicators / overlays menu for lightweight-charts (MA + RSI); does not include hosted TradingView widgets. */
export function PriceChartOverlayMenu({
  showSma7,
  showSma25,
  showRsi,
  onToggleSma7,
  onToggleSma25,
  onToggleRsi,
}: PriceChartOverlayMenuProps) {
  const [open, setOpen] = useState(false)
  const wrapRef = useRef<HTMLDivElement>(null)
  const menuId = useId()

  useEffect(() => {
    if (!open) return
    const onDoc = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onDoc)
    return () => document.removeEventListener('mousedown', onDoc)
  }, [open])

  const activeCount = (showSma7 ? 1 : 0) + (showSma25 ? 1 : 0) + (showRsi ? 1 : 0)
  const label = activeCount > 0 ? `Indicators (${activeCount})` : 'Indicators'

  return (
    <div className="relative" ref={wrapRef}>
      <button
        type="button"
        data-testid="price-chart-indicators-trigger"
        className={`tab-neo !text-[10px] !px-2 !py-1 ${open ? 'tab-neo-active' : 'tab-neo-inactive'}`}
        aria-expanded={open}
        aria-controls={menuId}
        aria-haspopup="true"
        aria-label="Chart indicators and overlays"
        onClick={() => {
          sounds.playButtonPress()
          setOpen((o) => !o)
        }}
      >
        {label}
      </button>
      {open && (
        <div
          id={menuId}
          data-testid="price-chart-indicators-panel"
          role="group"
          aria-label="Toggle chart overlays"
          className="absolute right-0 top-[calc(100%+6px)] z-20 min-w-[200px] rounded-md border border-white/15 p-3 shadow-lg backdrop-blur-sm"
          style={{ background: 'var(--panel-bg-strong)' }}
        >
          <div
            className="text-[10px] uppercase tracking-wide font-semibold mb-2"
            style={{ color: 'var(--ink-subtle)' }}
          >
            Overlays
          </div>
          <label className="flex items-center gap-2 cursor-pointer py-1.5 text-sm" style={{ color: 'var(--ink)' }}>
            <input
              type="checkbox"
              checked={showSma7}
              onChange={() => {
                sounds.playButtonPress()
                onToggleSma7()
              }}
              className="rounded border-white/30"
            />
            MA 7
          </label>
          <label className="flex items-center gap-2 cursor-pointer py-1.5 text-sm" style={{ color: 'var(--ink)' }}>
            <input
              type="checkbox"
              checked={showSma25}
              onChange={() => {
                sounds.playButtonPress()
                onToggleSma25()
              }}
              className="rounded border-white/30"
            />
            MA 25
          </label>
          <label className="flex items-center gap-2 cursor-pointer py-1.5 text-sm" style={{ color: 'var(--ink)' }}>
            <input
              type="checkbox"
              checked={showRsi}
              onChange={() => {
                sounds.playButtonPress()
                onToggleRsi()
              }}
              className="rounded border-white/30"
            />
            RSI 14
          </label>
        </div>
      )}
    </div>
  )
}
