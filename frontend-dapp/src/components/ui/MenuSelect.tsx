import { useCallback, useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

export interface MenuSelectOption {
  value: string
  label: string
}

export interface MenuSelectProps {
  /** Associates with an external `<label htmlFor={id}>`. */
  id?: string
  value: string
  options: MenuSelectOption[]
  onChange: (value: string) => void
  disabled?: boolean
  'aria-label'?: string
  className?: string
  /** Shown on the trigger when there are no options. */
  emptyLabel?: string
}

/**
 * Custom listbox + portal menu (same stacking pattern as TokenSelect).
 * Replaces native `<select>` where OS pickers break layout / z-order on mobile and tablet.
 */
export function MenuSelect({
  id,
  value,
  options,
  onChange,
  disabled,
  'aria-label': ariaLabel,
  className,
  emptyLabel = 'No options',
}: MenuSelectProps) {
  const [open, setOpen] = useState(false)
  const [dropdownPos, setDropdownPos] = useState<{
    top: number
    left: number
    width: number
    maxHeight: number
  } | null>(null)
  const rootRef = useRef<HTMLDivElement>(null)
  const dropdownRef = useRef<HTMLUListElement>(null)
  const listId = useId()

  const canOpen = options.length > 0 && !disabled
  const selectedLabel = useMemo(() => {
    const hit = options.find((o) => o.value === value)
    return hit?.label ?? value
  }, [options, value])

  const close = useCallback(() => setOpen(false), [])

  const updateDropdownPosition = useCallback(() => {
    const el = rootRef.current
    if (!el) return
    const r = el.getBoundingClientRect()
    const gap = 8
    const preferredMax = 280
    const spaceBelow = window.innerHeight - r.bottom - gap - 8
    const maxHeight = Math.min(preferredMax, Math.max(120, spaceBelow))
    setDropdownPos({
      top: r.bottom + gap,
      left: r.left,
      width: r.width,
      maxHeight,
    })
  }, [])

  useLayoutEffect(() => {
    if (!open || !canOpen) {
      setDropdownPos(null)
      return
    }
    updateDropdownPosition()
    const w = window
    w.addEventListener('scroll', updateDropdownPosition, true)
    w.addEventListener('resize', updateDropdownPosition)
    return () => {
      w.removeEventListener('scroll', updateDropdownPosition, true)
      w.removeEventListener('resize', updateDropdownPosition)
    }
  }, [open, canOpen, updateDropdownPosition])

  useEffect(() => {
    if (!open) return
    function onDocMouseDown(e: MouseEvent) {
      const t = e.target as Node
      if (rootRef.current?.contains(t)) return
      if (dropdownRef.current?.contains(t)) return
      close()
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') close()
    }
    document.addEventListener('mousedown', onDocMouseDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDocMouseDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [open, close])

  return (
    <div ref={rootRef} className={className ?? 'relative w-full'}>
      <button
        type="button"
        id={id}
        disabled={!canOpen}
        className="token-select-trigger"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => {
          if (!canOpen) return
          setOpen((o) => !o)
        }}
      >
        <span className="truncate flex-1 text-left">{canOpen ? selectedLabel : emptyLabel}</span>
        <span className="token-select-chevron shrink-0" aria-hidden />
      </button>

      {open &&
        canOpen &&
        dropdownPos &&
        createPortal(
          <ul
            ref={dropdownRef}
            id={listId}
            role="listbox"
            className="token-select-dropdown"
            aria-label={ariaLabel}
            style={{
              position: 'fixed',
              top: dropdownPos.top,
              left: dropdownPos.left,
              width: dropdownPos.width,
              maxHeight: dropdownPos.maxHeight,
            }}
          >
            {options.map((opt) => {
              const isSelected = opt.value === value
              return (
                <li key={opt.value} role="none">
                  <button
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    className={`token-select-option ${isSelected ? 'token-select-option-active' : ''}`}
                    onClick={() => {
                      onChange(opt.value)
                      close()
                    }}
                  >
                    <span className="truncate">{opt.label}</span>
                  </button>
                </li>
              )
            })}
          </ul>,
          document.body
        )}
    </div>
  )
}
