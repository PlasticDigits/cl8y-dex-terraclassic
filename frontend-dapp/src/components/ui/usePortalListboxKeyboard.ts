import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type RefObject,
} from 'react'
import {
  findPortalListboxTypeaheadIndex,
  movePortalListboxActiveIndex,
  portalListboxOptionId,
  PORTAL_LISTBOX_TYPEAHEAD_RESET_MS,
} from './portalListboxKeyboard'

export type UsePortalListboxKeyboardArgs = {
  open: boolean
  canOpen: boolean
  optionCount: number
  /** Index of the currently selected value in the option list, or -1 when none. */
  selectedIndex: number
  /** Label used for typeahead matching (symbol prefix for tokens, option label for menus). */
  getTypeaheadLabel: (index: number) => string
  listId: string
  triggerRef: RefObject<HTMLButtonElement | null>
  listboxRef: RefObject<HTMLElement | null>
  onOpen: () => void
  onClose: () => void
  onSelectIndex: (index: number) => void
}

function isPrintableKey(key: string): boolean {
  return key.length === 1 && key !== ' '
}

function wrapActive(index: number, length: number): number {
  if (length <= 0) return 0
  return Math.min(Math.max(index, 0), length - 1)
}

export function usePortalListboxKeyboard({
  open,
  canOpen,
  optionCount,
  selectedIndex,
  getTypeaheadLabel,
  listId,
  triggerRef,
  listboxRef,
  onOpen,
  onClose,
  onSelectIndex,
}: UsePortalListboxKeyboardArgs) {
  const [activeIndex, setActiveIndex] = useState(0)
  const activeIndexRef = useRef(0)
  const typeaheadRef = useRef('')
  const typeaheadTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const selectingRef = useRef(false)
  const getTypeaheadLabelRef = useRef(getTypeaheadLabel)

  useEffect(() => {
    getTypeaheadLabelRef.current = getTypeaheadLabel
  }, [getTypeaheadLabel])

  useEffect(() => {
    activeIndexRef.current = activeIndex
  }, [activeIndex])

  const clearTypeahead = useCallback(() => {
    typeaheadRef.current = ''
    if (typeaheadTimerRef.current) {
      clearTimeout(typeaheadTimerRef.current)
      typeaheadTimerRef.current = null
    }
  }, [])

  const scheduleTypeaheadReset = useCallback(() => {
    if (typeaheadTimerRef.current) clearTimeout(typeaheadTimerRef.current)
    typeaheadTimerRef.current = setTimeout(() => {
      typeaheadRef.current = ''
      typeaheadTimerRef.current = null
    }, PORTAL_LISTBOX_TYPEAHEAD_RESET_MS)
  }, [])

  const closeAndRestoreFocus = useCallback(() => {
    clearTypeahead()
    onClose()
    triggerRef.current?.focus()
  }, [clearTypeahead, onClose, triggerRef])

  const selectIndex = useCallback(
    (index: number) => {
      if (optionCount <= 0 || index < 0 || index >= optionCount) return
      if (selectingRef.current) return
      selectingRef.current = true
      onSelectIndex(index)
      closeAndRestoreFocus()
    },
    [closeAndRestoreFocus, onSelectIndex, optionCount]
  )

  useEffect(() => {
    if (!open) selectingRef.current = false
  }, [open])

  const applyTypeahead = useCallback(
    (char: string, fromIndex: number) => {
      if (optionCount <= 0) return
      const nextQuery = typeaheadRef.current + char
      typeaheadRef.current = nextQuery
      scheduleTypeaheadReset()
      const labels = Array.from({ length: optionCount }, (_, i) => getTypeaheadLabelRef.current(i))
      const match = findPortalListboxTypeaheadIndex(labels, nextQuery, fromIndex)
      if (match !== null) setActiveIndex(match)
    },
    [optionCount, scheduleTypeaheadReset]
  )

  const setActiveForOpen = useCallback(
    (index: number) => {
      const next = wrapActive(index, optionCount)
      activeIndexRef.current = next
      setActiveIndex(next)
    },
    [optionCount]
  )

  const openWithActiveIndex = useCallback(
    (index: number) => {
      if (!canOpen || optionCount <= 0) return
      clearTypeahead()
      setActiveForOpen(index)
      onOpen()
    },
    [canOpen, clearTypeahead, onOpen, optionCount, setActiveForOpen]
  )

  /** Call before toggling open via pointer so the active row matches the selection. */
  const primeActiveIndexForClickOpen = useCallback(() => {
    setActiveForOpen(selectedIndex >= 0 ? selectedIndex : 0)
  }, [selectedIndex, setActiveForOpen])

  const handleOpenListboxKey = useCallback(
    (key: string) => {
      if (optionCount <= 0) return

      switch (key) {
        case 'ArrowDown':
          setActiveIndex((i) => movePortalListboxActiveIndex(i, 1, optionCount))
          break
        case 'ArrowUp':
          setActiveIndex((i) => movePortalListboxActiveIndex(i, -1, optionCount))
          break
        case 'Home':
          setActiveIndex(0)
          break
        case 'End':
          setActiveIndex(optionCount - 1)
          break
        case 'Enter':
        case ' ':
          selectIndex(activeIndexRef.current)
          break
        case 'Tab':
          closeAndRestoreFocus()
          break
        default:
          if (isPrintableKey(key)) {
            applyTypeahead(key, activeIndexRef.current)
          }
      }
    },
    [applyTypeahead, closeAndRestoreFocus, optionCount, selectIndex]
  )

  const handleTriggerKeyDown = useCallback(
    (e: ReactKeyboardEvent<HTMLButtonElement>) => {
      if (!canOpen || optionCount <= 0 || open) return

      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          openWithActiveIndex(0)
          break
        case 'ArrowUp':
          e.preventDefault()
          openWithActiveIndex(optionCount - 1)
          break
        case 'Home':
          e.preventDefault()
          openWithActiveIndex(0)
          break
        case 'End':
          e.preventDefault()
          openWithActiveIndex(optionCount - 1)
          break
        default:
          if (isPrintableKey(e.key)) {
            e.preventDefault()
            clearTypeahead()
            setActiveForOpen(0)
            onOpen()
            applyTypeahead(e.key, -1)
          }
      }
    },
    [applyTypeahead, canOpen, clearTypeahead, onOpen, open, openWithActiveIndex, optionCount, setActiveForOpen]
  )

  useEffect(() => {
    if (!open) {
      clearTypeahead()
      return
    }
    if (optionCount <= 0) return
    const id = requestAnimationFrame(() => {
      listboxRef.current?.focus()
    })
    return () => cancelAnimationFrame(id)
  }, [clearTypeahead, listboxRef, open, optionCount])

  useEffect(() => {
    if (!open || optionCount <= 0) return
    const optionId = portalListboxOptionId(listId, activeIndex)
    const el = document.getElementById(optionId)
    if (typeof el?.scrollIntoView === 'function') {
      el.scrollIntoView({ block: 'nearest' })
    }
  }, [activeIndex, listId, open, optionCount])

  useEffect(() => {
    if (!open || optionCount <= 0) return

    function onDocKey(e: KeyboardEvent) {
      const target = e.target as Node | null
      if (target && !triggerRef.current?.contains(target) && !listboxRef.current?.contains(target)) {
        return
      }

      if (e.key === 'Escape') {
        e.preventDefault()
        closeAndRestoreFocus()
        return
      }

      if (
        e.key === 'ArrowDown' ||
        e.key === 'ArrowUp' ||
        e.key === 'Home' ||
        e.key === 'End' ||
        e.key === 'Enter' ||
        e.key === ' ' ||
        isPrintableKey(e.key)
      ) {
        e.preventDefault()
        handleOpenListboxKey(e.key)
      }
    }

    document.addEventListener('keydown', onDocKey)
    return () => document.removeEventListener('keydown', onDocKey)
  }, [closeAndRestoreFocus, handleOpenListboxKey, listboxRef, open, optionCount, triggerRef])

  useEffect(() => () => clearTypeahead(), [clearTypeahead])

  const activeOptionId =
    open && optionCount > 0 && activeIndex >= 0 && activeIndex < optionCount
      ? portalListboxOptionId(listId, activeIndex)
      : undefined

  const getOptionClassName = useCallback(
    (index: number, isSelected: boolean) => {
      const active = open && index === activeIndex
      return [
        'token-select-option',
        isSelected ? 'token-select-option-active' : '',
        active ? 'token-select-option-keyboard-active' : '',
      ]
        .filter(Boolean)
        .join(' ')
    },
    [activeIndex, open]
  )

  return {
    activeIndex,
    activeOptionId,
    getOptionClassName,
    getOptionId: (index: number) => portalListboxOptionId(listId, index),
    handleTriggerKeyDown,
    primeActiveIndexForClickOpen,
  }
}
