import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useQuery } from '@tanstack/react-query'
import { usePortalListbox } from '@/components/ui/PortalListbox'
import { movePortalListboxActiveIndex, portalListboxOptionId } from '@/components/ui/portalListboxKeyboard'
import { getPairs } from '@/services/indexer/client'
import type { IndexerPair } from '@/types'
import type { PairInfo } from '@/types'
import { pairInfoMenuLabel, indexerPairMenuLabel, type PairMenuLabelVariant } from '@/utils/pairMenuOptions'
import {
  filterFactoryPairsByLocalSearch,
  isPairSearchQueryReady,
  PAIR_SEARCH_RESULT_LIMIT,
} from '@/utils/pairSearchQuery'
import { formatQuoteVolume24h, getDecimals } from '@/utils/formatAmount'
import {
  filterRetailDiscoveryIndexerPairs,
  filterRetailDiscoveryPairInfos,
  isTestPair,
  pairInfoLegIds,
  pairInfoLegSymbols,
  sortPairInfosByCatalog,
  type PairCatalogVolume,
} from '@/utils/pairCatalogRank'

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
  /** Closed-control label only. Does not invert on combobox click (GitLab #524). */
  selectedLabelOverride?: string
}

type PairSearchOption = {
  value: string
  label: string
  volumeQuote24h?: string
  quoteDecimals?: number
  isTestPair?: boolean
}

function indexerPairToOption(p: IndexerPair, variant: PairMenuLabelVariant): PairSearchOption {
  return {
    value: p.pair_address,
    label: indexerPairMenuLabel(p, { variant }),
    volumeQuote24h: p.volume_quote_24h,
    quoteDecimals: p.asset_1.decimals,
    isTestPair: isTestPair(
      p.asset_0.symbol,
      p.asset_1.symbol,
      p.asset_0.contract_addr ?? p.asset_0.denom ?? undefined,
      p.asset_1.contract_addr ?? p.asset_1.denom ?? undefined
    ),
  }
}

function factoryPairToOption(p: PairInfo, variant: PairMenuLabelVariant, volume?: PairCatalogVolume): PairSearchOption {
  const [id0, id1] = pairInfoLegIds(p)
  const [sym0, sym1] = pairInfoLegSymbols(p)
  return {
    value: p.contract_addr,
    label: pairInfoMenuLabel(p, { variant }),
    volumeQuote24h: volume?.raw ?? undefined,
    quoteDecimals: volume?.quoteDecimals ?? getDecimals(p.asset_infos[1]),
    isTestPair: isTestPair(sym0, sym1, id0, id1),
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
  selectedLabelOverride,
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
  const [indexerUnavailable, setIndexerUnavailable] = useState(false)

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
    if (selectedLabelOverride) return selectedLabelOverride
    return labelsByAddress.get(value) ?? value
  }, [value, labelsByAddress, selectedLabelOverride])

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(searchText.trim()), 300)
    return () => clearTimeout(timer)
  }, [searchText])

  useEffect(() => {
    activeIndexRef.current = activeIndex
  }, [activeIndex])

  const queryReady = isPairSearchQueryReady(debouncedSearch)
  const useIndexerSearch = queryReady && open && !indexerUnavailable
  const inputValue = open ? searchText : selectedLabel

  const emptyQueryLimit = Math.min(100, Math.max(PAIR_SEARCH_RESULT_LIMIT, factoryPairs.length))

  const pairsQuery = useQuery({
    queryKey: ['pair-search', debouncedSearch, open, emptyQueryLimit],
    queryFn: () => {
      const q = debouncedSearch || undefined
      return getPairs({
        q,
        sort: q ? 'relevance' : 'volume_24h',
        order: 'desc',
        limit: q ? PAIR_SEARCH_RESULT_LIMIT : emptyQueryLimit,
      })
    },
    enabled: useIndexerSearch && factoryPairs.length > 0 && !disabled,
    staleTime: 30_000,
    retry: false,
  })

  useEffect(() => {
    if (pairsQuery.isError) setIndexerUnavailable(true)
  }, [pairsQuery.isError])

  const useLocalFallback = indexerUnavailable || pairsQuery.isError

  const localSearchQuery = useMemo(() => {
    if (useLocalFallback && open) return debouncedSearch || searchText.trim()
    return debouncedSearch
  }, [useLocalFallback, open, debouncedSearch, searchText])

  const localFallbackPairs = useMemo(() => {
    if (!useLocalFallback || factoryPairs.length === 0) return []
    return filterFactoryPairsByLocalSearch(factoryPairs, localSearchQuery, PAIR_SEARCH_RESULT_LIMIT, variant)
  }, [useLocalFallback, factoryPairs, localSearchQuery, variant])

  const indexerByAddress = useMemo(() => {
    const map = new Map<string, IndexerPair>()
    for (const p of pairsQuery.data?.items ?? []) {
      if (factorySet.has(p.pair_address)) map.set(p.pair_address, p)
    }
    return map
  }, [pairsQuery.data, factorySet])

  const catalogEmptyOptions = useMemo(() => {
    const volumeByAddress = new Map<string, PairCatalogVolume>()
    for (const [addr, p] of indexerByAddress) {
      volumeByAddress.set(addr, { raw: p.volume_quote_24h, quoteDecimals: p.asset_1.decimals })
    }
    return sortPairInfosByCatalog(filterRetailDiscoveryPairInfos(factoryPairs), volumeByAddress)
      .slice(0, PAIR_SEARCH_RESULT_LIMIT)
      .map((p) => {
        const indexed = indexerByAddress.get(p.contract_addr)
        return indexed
          ? indexerPairToOption(indexed, variant)
          : factoryPairToOption(p, variant, volumeByAddress.get(p.contract_addr))
      })
  }, [factoryPairs, indexerByAddress, variant])

  const options: PairSearchOption[] = useMemo(() => {
    if (factoryPairs.length === 0) return []

    let result: PairSearchOption[]

    if (!debouncedSearch) {
      // Browse list: factory universe + catalog rank (economic first). Do not wait on indexer.
      result = useLocalFallback ? localFallbackPairs.map((p) => factoryPairToOption(p, variant)) : catalogEmptyOptions
    } else if (useLocalFallback) {
      result = localFallbackPairs.map((p) => factoryPairToOption(p, variant))
    } else if (!pairsQuery.data) {
      result = []
    } else {
      const fromIndexer = filterRetailDiscoveryIndexerPairs(
        pairsQuery.data.items.filter((p) => factorySet.has(p.pair_address))
      ).map((p) => indexerPairToOption(p, variant))
      result = fromIndexer
    }

    // Empty query: keep current pair in the list so Enter without typing re-selects it (#350).
    // Typed query: omit prepend so Enter commits the first search hit, not the current pair.
    if (!debouncedSearch && value && factorySet.has(value) && !result.some((o) => o.value === value)) {
      const indexed = indexerByAddress.get(value)
      if (indexed) {
        result = [indexerPairToOption(indexed, variant), ...result]
      } else {
        const factory = factoryPairs.find((p) => p.contract_addr === value)
        if (factory) result = [factoryPairToOption(factory, variant), ...result]
      }
    }
    return result
  }, [
    factoryPairs,
    useLocalFallback,
    localFallbackPairs,
    pairsQuery.data,
    factorySet,
    variant,
    debouncedSearch,
    value,
    catalogEmptyOptions,
    indexerByAddress,
  ])

  const canOpen = factoryPairs.length > 0 && !disabled
  const selectedIndex = useMemo(() => options.findIndex((o) => o.value === value), [options, value])

  useEffect(() => {
    if (!open) return
    if (debouncedSearch) {
      setActiveIndex(0)
    } else {
      setActiveIndex(selectedIndex >= 0 ? selectedIndex : 0)
    }
  }, [open, selectedIndex, options.length, debouncedSearch])

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

  const awaitingIndexer =
    useIndexerSearch && !useLocalFallback && (pairsQuery.isLoading || (pairsQuery.isFetching && !pairsQuery.data))

  const showEmptyState =
    open && options.length === 0 && !awaitingIndexer && (queryReady || useLocalFallback) && localSearchQuery.length > 0

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
            aria-busy={awaitingIndexer || undefined}
            style={dropdownStyle}
          >
            {useLocalFallback && options.length > 0 ? (
              <li className="px-3 py-1.5 text-xs" style={{ color: 'var(--ink-dim)' }} role="presentation">
                Offline search — showing factory pairs from cached labels
              </li>
            ) : null}
            {awaitingIndexer && options.length === 0 ? (
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
              const showTestDivider =
                !debouncedSearch && opt.isTestPair && (index === 0 || !options[index - 1]?.isTestPair)
              const volLabel = formatQuoteVolume24h(opt.volumeQuote24h, opt.quoteDecimals ?? 6, 3)
              return (
                <li key={opt.value} role="none">
                  {showTestDivider ? (
                    <div
                      className="px-3 py-1.5 text-[10px] uppercase tracking-wide font-semibold"
                      style={{ color: 'var(--ink-dim)' }}
                      role="presentation"
                    >
                      Test pairs
                    </div>
                  ) : null}
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
                    {volLabel ? (
                      <span
                        className="shrink-0 text-[10px] uppercase tracking-wide font-medium px-1.5 py-0.5 rounded"
                        style={{ color: 'var(--ink-dim)', background: 'var(--surface-muted)' }}
                        title="24h quote volume (human units of the quote token)"
                      >
                        vol {volLabel}
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
