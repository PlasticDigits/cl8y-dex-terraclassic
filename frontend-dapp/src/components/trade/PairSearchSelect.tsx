import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useQuery } from '@tanstack/react-query'
import { usePortalListbox } from '@/components/ui/PortalListbox'
import { movePortalListboxActiveIndex, portalListboxOptionId } from '@/components/ui/portalListboxKeyboard'
import { getPairs } from '@/services/indexer/client'
import type { IndexerPair } from '@/types'
import type { PairInfo } from '@/types'
import { pairInfoMenuLabel, indexerPairMenuLabel, type PairMenuLabelVariant } from '@/utils/pairMenuOptions'
import { filterPairsByLocalSearch, isPairSearchQueryReady, PAIR_SEARCH_RESULT_LIMIT } from '@/utils/pairSearchQuery'
import { formatNum } from '@/utils/formatAmount'

export interface PairSearchSelectProps {
  id?: string
  value: string
  onChange: (value: string) => void
  /** Factory-registered pairs — results are limited to this set; used for fallback search. */
  factoryPairs: PairInfo[]
  variant?: PairMenuLabelVariant
  disabled?: boolean
  placeholder?: string
  /** Shown when factory has no pairs. */
  emptyLabel?: string
  'aria-label'?: string
  className?: string
  onOptionIntent?: (value: string) => void
}

type PairSearchOption = {
  value: string
  label: string
  volumeQuote24h?: string
}

function indexerPairToOption(p: IndexerPair, variant: PairMenuLabelVariant): PairSearchOption {
  return {
    value: p.pair_address,
    label: indexerPairMenuLabel(p, { variant }),
    volumeQuote24h: p.volume_quote_24h,
  }
}

function factoryPairToOption(p: PairInfo, variant: PairMenuLabelVariant): PairSearchOption {
  return {
    value: p.contract_addr,
    label: pairInfoMenuLabel(p, { variant }),
  }
}

export function PairSearchSelect({
  id,
  value,
  onChange,
  factoryPairs,
  variant = 'full',
  disabled,
  placeholder = 'Search pairs…',
  emptyLabel = 'No pairs on factory',
  'aria-label': ariaLabel,
  className,
  onOptionIntent,
}: PairSearchSelectProps) {
  const rootRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const dropdownRef = useRef<HTMLUListElement>(null)
  const listId = useId()

  const [open, setOpen] = useState(false)
  const [searchText, setSearchText] = useState('')
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)
  const activeIndexRef = useRef(0)

  const factorySet = useMemo(() => new Set(factoryPairs.map((p) => p.contract_addr)), [factoryPairs])

  const labelsByAddress = useMemo(() => {
    const map = new Map<string, string>()
    for (const p of factoryPairs) {
      map.set(p.contract_addr, pairInfoMenuLabel(p, { variant }))
    }
    return map
  }, [factoryPairs, variant])

  const selectedLabel = useMemo(() => {
    if (!value) return ''
    return labelsByAddress.get(value) ?? value
  }, [value, labelsByAddress])

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(searchText.trim()), 300)
    return () => clearTimeout(timer)
  }, [searchText])

  useEffect(() => {
    activeIndexRef.current = activeIndex
  }, [activeIndex])

  const queryReady = isPairSearchQueryReady(debouncedSearch)
  const useIndexerSearch = queryReady && open
  const inputValue = open ? searchText : selectedLabel

  const pairsQuery = useQuery({
    queryKey: ['pair-search', debouncedSearch, open],
    queryFn: () => {
      const q = debouncedSearch || undefined
      return getPairs({
        q,
        sort: q ? 'relevance' : 'volume_24h',
        order: 'desc',
        limit: PAIR_SEARCH_RESULT_LIMIT,
      })
    },
    enabled: useIndexerSearch && factoryPairs.length > 0 && !disabled,
    staleTime: 30_000,
    retry: false,
  })

  const fallbackAddresses = useMemo(() => {
    if (!pairsQuery.isError) return []
    return filterPairsByLocalSearch(labelsByAddress, debouncedSearch, PAIR_SEARCH_RESULT_LIMIT).filter((addr) =>
      factorySet.has(addr)
    )
  }, [pairsQuery.isError, labelsByAddress, debouncedSearch, factorySet])

  const options: PairSearchOption[] = useMemo(() => {
    if (factoryPairs.length === 0) return []

    let result: PairSearchOption[]
    if (pairsQuery.isError) {
      result = fallbackAddresses.map((addr) => {
        const factory = factoryPairs.find((p) => p.contract_addr === addr)
        return factory ? factoryPairToOption(factory, variant) : { value: addr, label: addr }
      })
    } else if (!pairsQuery.data) {
      result = []
    } else {
      const fromIndexer = pairsQuery.data.items
        .filter((p) => factorySet.has(p.pair_address))
        .map((p) => indexerPairToOption(p, variant))
      if (fromIndexer.length > 0) {
        result = fromIndexer
      } else if (debouncedSearch) {
        result = []
      } else {
        result = factoryPairs.slice(0, PAIR_SEARCH_RESULT_LIMIT).map((p) => factoryPairToOption(p, variant))
      }
    }

    if (value && factorySet.has(value) && !result.some((o) => o.value === value)) {
      const factory = factoryPairs.find((p) => p.contract_addr === value)
      if (factory) {
        result = [factoryPairToOption(factory, variant), ...result]
      }
    }
    return result
  }, [
    factoryPairs,
    pairsQuery.isError,
    pairsQuery.data,
    fallbackAddresses,
    factorySet,
    variant,
    debouncedSearch,
    value,
  ])

  const canOpen = factoryPairs.length > 0 && !disabled
  const selectedIndex = useMemo(() => options.findIndex((o) => o.value === value), [options, value])

  useEffect(() => {
    if (open) setActiveIndex(selectedIndex >= 0 ? selectedIndex : 0)
  }, [open, selectedIndex, options.length])

  const close = useCallback(() => {
    setOpen(false)
    setSearchText('')
    setDebouncedSearch('')
  }, [])

  const selectIndex = useCallback(
    (index: number) => {
      const opt = options[index]
      if (!opt?.value) return
      onChange(opt.value)
      close()
      inputRef.current?.blur()
    },
    [options, onChange, close]
  )

  const dropdownStyle = usePortalListbox({
    open,
    canShow: canOpen && open,
    anchorRef: rootRef,
    dropdownRef,
    onClose: close,
    preferredMaxHeight: 280,
  })

  const handleInputKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (!canOpen) return
      if (!open && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
        e.preventDefault()
        setOpen(true)
        return
      }
      if (!open) return
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          setActiveIndex((i) => movePortalListboxActiveIndex(i, 1, options.length))
          break
        case 'ArrowUp':
          e.preventDefault()
          setActiveIndex((i) => movePortalListboxActiveIndex(i, -1, options.length))
          break
        case 'Home':
          e.preventDefault()
          setActiveIndex(0)
          break
        case 'End':
          e.preventDefault()
          setActiveIndex(Math.max(0, options.length - 1))
          break
        case 'Enter':
          e.preventDefault()
          if (options.length > 0) selectIndex(activeIndexRef.current)
          break
        case 'Escape':
          e.preventDefault()
          close()
          break
        case 'Tab':
          close()
          break
        default:
          break
      }
    },
    [canOpen, open, options.length, selectIndex, close]
  )

  const showEmptyState =
    open && options.length === 0 && !pairsQuery.isLoading && queryReady && debouncedSearch.length > 0
  const activeOptionId = open && options.length > 0 ? portalListboxOptionId(listId, activeIndex) : undefined

  return (
    <div ref={rootRef} className={`token-select-root ${className ?? 'relative w-full'}`}>
      <input
        ref={inputRef}
        type="text"
        id={id}
        role="combobox"
        className="token-select-trigger w-full"
        aria-label={ariaLabel}
        aria-autocomplete="list"
        aria-expanded={open}
        aria-controls={listId}
        aria-activedescendant={activeOptionId}
        disabled={!canOpen}
        placeholder={canOpen ? placeholder : emptyLabel}
        value={inputValue}
        onChange={(e) => {
          setSearchText(e.target.value)
          if (!open) setOpen(true)
        }}
        onFocus={() => {
          if (!canOpen) return
          setOpen(true)
        }}
        onKeyDown={handleInputKeyDown}
        onBlur={() => {
          window.setTimeout(() => {
            if (!rootRef.current?.contains(document.activeElement)) close()
          }, 150)
        }}
      />
      <span
        className="token-select-chevron pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 shrink-0"
        aria-hidden
      />

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
            aria-busy={pairsQuery.isLoading || undefined}
            style={dropdownStyle}
          >
            {(pairsQuery.isLoading || (useIndexerSearch && !pairsQuery.data && !pairsQuery.isError)) &&
            options.length === 0 ? (
              <li className="px-3 py-2 text-sm" style={{ color: 'var(--ink-dim)' }} role="presentation">
                Searching…
              </li>
            ) : null}
            {showEmptyState ? (
              <li className="px-3 py-2 text-sm" style={{ color: 'var(--ink-dim)' }} role="presentation">
                No pairs match your search
              </li>
            ) : null}
            {options.map((opt, index) => {
              const isSelected = opt.value === value
              const isActive = index === activeIndex
              return (
                <li key={opt.value} role="none">
                  <button
                    type="button"
                    id={portalListboxOptionId(listId, index)}
                    role="option"
                    aria-selected={isSelected}
                    className={`token-select-option w-full flex items-center justify-between gap-2${isSelected ? ' token-select-option-active' : ''}${isActive ? ' token-select-option-keyboard-active' : ''}`}
                    onPointerEnter={() => {
                      setActiveIndex(index)
                      onOptionIntent?.(opt.value)
                    }}
                    onFocus={() => onOptionIntent?.(opt.value)}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => selectIndex(index)}
                  >
                    <span className="truncate text-left">{opt.label}</span>
                    {opt.volumeQuote24h && Number(opt.volumeQuote24h) > 0 ? (
                      <span
                        className="shrink-0 text-[10px] uppercase tracking-wide font-medium px-1.5 py-0.5 rounded"
                        style={{ color: 'var(--ink-dim)', background: 'var(--surface-muted)' }}
                        title="24h quote volume (indexed)"
                      >
                        vol {formatNum(opt.volumeQuote24h, 3)}
                      </span>
                    ) : null}
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
