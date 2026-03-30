import { useCallback, useId, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { usePortalListbox } from './PortalListbox'

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
 * Custom listbox + portal menu via {@link usePortalListbox} (shared with TokenSelect).
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
  const rootRef = useRef<HTMLDivElement>(null)
  const dropdownRef = useRef<HTMLUListElement>(null)
  const listId = useId()

  const canOpen = options.length > 0 && !disabled
  const selectedLabel = useMemo(() => {
    const hit = options.find((o) => o.value === value)
    return hit?.label ?? value
  }, [options, value])

  const close = useCallback(() => setOpen(false), [])

  const dropdownStyle = usePortalListbox({
    open,
    canShow: canOpen,
    anchorRef: rootRef,
    dropdownRef,
    onClose: close,
    preferredMaxHeight: 280,
  })

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
        dropdownStyle &&
        createPortal(
          <ul
            ref={dropdownRef}
            id={listId}
            role="listbox"
            className="token-select-dropdown"
            aria-label={ariaLabel}
            style={dropdownStyle}
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
