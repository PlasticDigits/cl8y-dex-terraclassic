import { useEffect, useId, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

/** Tap, hover or keyboard focus reveals details without persistent metric chrome. */
export function ProtocolDetails({ detail, label = 'Liquidity details' }: { detail: string; label?: string }) {
  const id = useId()
  const button = useRef<HTMLButtonElement>(null)
  const [open, setOpen] = useState(false)
  const [position, setPosition] = useState({ left: 16, top: 0, width: 280 })

  function reveal() {
    const bounds = button.current?.getBoundingClientRect()
    if (bounds) {
      const width = Math.min(280, window.innerWidth - 32)
      setPosition({
        left: Math.max(16, Math.min(bounds.left, window.innerWidth - width - 16)),
        top: bounds.bottom + 88 > window.innerHeight ? Math.max(8, bounds.top - 88) : bounds.bottom + 8,
        width,
      })
    }
    setOpen(true)
  }

  useEffect(() => {
    if (!open) return
    function closeOutside(event: PointerEvent) {
      if (!button.current?.contains(event.target as Node)) setOpen(false)
    }
    function closeEscape(event: KeyboardEvent) {
      if (event.key === 'Escape') setOpen(false)
    }
    function closeScroll() {
      setOpen(false)
    }
    document.addEventListener('pointerdown', closeOutside)
    document.addEventListener('keydown', closeEscape)
    window.addEventListener('scroll', closeScroll, true)
    window.addEventListener('resize', closeScroll)
    return () => {
      document.removeEventListener('pointerdown', closeOutside)
      document.removeEventListener('keydown', closeEscape)
      window.removeEventListener('scroll', closeScroll, true)
      window.removeEventListener('resize', closeScroll)
    }
  }, [open])

  return (
    <>
      <button
        ref={button}
        type="button"
        aria-label={label}
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        aria-describedby={open ? id : undefined}
        className="inline-flex h-6 w-6 -my-1 shrink-0 items-center justify-center rounded-full"
        style={{ color: 'var(--ink-subtle)' }}
        onClick={reveal}
        onFocus={reveal}
        onBlur={() => setOpen(false)}
        onMouseEnter={reveal}
        onMouseLeave={() => {
          if (document.activeElement !== button.current) setOpen(false)
        }}
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <circle cx="8" cy="8" r="6" stroke="currentColor" />
          <path d="M8 7v4" stroke="currentColor" strokeLinecap="round" />
          <circle cx="8" cy="4.5" r=".7" fill="currentColor" />
        </svg>
      </button>
      {open &&
        createPortal(
          <div
            id={id}
            role="tooltip"
            className="fixed z-[10000] rounded-md border px-3 py-2 text-xs normal-case shadow-lg"
            style={{
              ...position,
              background: 'var(--surface-raised)',
              borderColor: 'var(--line)',
              color: 'var(--ink)',
            }}
          >
            {detail}
          </div>,
          document.body
        )}
    </>
  )
}
