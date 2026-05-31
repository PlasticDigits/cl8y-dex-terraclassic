import { useCallback, useId, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { usePortalListbox } from './PortalListbox'
import { usePortalListboxKeyboard } from './usePortalListboxKeyboard'

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
  /** Fired when the user hovers or focuses a list option — use to prefetch pair data before click (GitLab #180). */
  onOptionIntent?: (value: string) => void
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
  onOptionIntent,
}: MenuSelectProps) {
  const [open, setOpen] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const dropdownRef = useRef<HTMLUListElement>(null)
  const listId = useId()

  const canOpen = options.length > 0 && !disabled
  const selectedLabel = useMemo(() => {
    const hit = options.find((o) => o.value === value)
    return hit?.label ?? value
  }, [options, value])

  const close = useCallback(() => setOpen(false), [])
  const openMenu = useCallback(() => setOpen(true), [])

  const selectedIndex = useMemo(() => options.findIndex((o) => o.value === value), [options, value])

  const getTypeaheadLabel = useCallback((index: number) => options[index]?.label ?? '', [options])

  const { activeOptionId, getOptionClassName, getOptionId, handleTriggerKeyDown, primeActiveIndexForClickOpen } =
    usePortalListboxKeyboard({
      open,
      canOpen,
      optionCount: options.length,
      selectedIndex,
      getTypeaheadLabel,
      listId,
      triggerRef,
      listboxRef: dropdownRef,
      onOpen: openMenu,
      onClose: close,
      onSelectIndex: (index) => {
        const next = options[index]!.value
        if (next !== value) onChange(next)
      },
    })

  const dropdownStyle = usePortalListbox({
    open,
    canShow: canOpen,
    anchorRef: rootRef,
    dropdownRef,
    onClose: close,
    preferredMaxHeight: 280,
  })

  return (
    <div ref={rootRef} className={`token-select-root ${className ?? 'relative w-full'}`}>
      <button
        ref={triggerRef}
        type="button"
        id={id}
        disabled={!canOpen}
        className="token-select-trigger"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onKeyDown={handleTriggerKeyDown}
        onClick={() => {
          if (!canOpen) return
          setOpen((o) => {
            if (!o) primeActiveIndexForClickOpen()
            return !o
          })
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
            tabIndex={-1}
            className="token-select-dropdown"
            aria-label={ariaLabel}
            aria-activedescendant={activeOptionId}
            style={dropdownStyle}
          >
            {options.map((opt, index) => {
              const isSelected = opt.value === value
              return (
                <li key={opt.value} role="none">
                  <button
                    type="button"
                    id={getOptionId(index)}
                    role="option"
                    aria-selected={isSelected}
                    className={getOptionClassName(index, isSelected)}
                    onPointerEnter={() => onOptionIntent?.(opt.value)}
                    onFocus={() => onOptionIntent?.(opt.value)}
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
