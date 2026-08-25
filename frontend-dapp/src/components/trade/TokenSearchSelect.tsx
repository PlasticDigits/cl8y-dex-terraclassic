import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { usePortalListbox } from '@/components/ui/PortalListbox'
import { movePortalListboxActiveIndex, portalListboxOptionId } from '@/components/ui/portalListboxKeyboard'
import { SearchSelectMenuSearch } from '@/components/ui/SearchSelectMenuSearch'
import { TokenLogo } from '@/components/ui/TokenLogo'
import { useCoarseNarrowViewport } from '@/hooks/useCoarseNarrowViewport'
import type { AssetInfo } from '@/types'
import { tokenAssetInfo } from '@/types'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'
import { optionMovedBeyondThreshold } from '@/lib/optionTapStability'
import { getAddressForBlockie, getTokenLogoURI } from '@/utils/tokenDisplay'
import { resolveTrustedTokenLogoUrl } from '@/utils/tokenLogoAllowlist'
import {
  filterTokensByLocalSearch,
  isTokenSearchQueryReady,
  normalizeTokenSearchQuery,
  TOKEN_SEARCH_DEBOUNCE_MS,
  TOKEN_SEARCH_MAX_QUERY_LENGTH,
} from '@/utils/tokenSearchQuery'

export interface TokenSearchSelectProps {
  value: string
  /** Factory-routable token ids — results are limited to this set (GitLab #481). */
  tokens: string[]
  onChange: (tokenId: string) => void
  excludeToken?: string
  'aria-label': string
  disabled?: boolean
  loadingLabel?: string
  placeholder?: string
  className?: string
  id?: string
}

function logoPropsForToken(tokenId: string): {
  logoURI: string | undefined
  addressForBlockie: string | undefined
  blockieSeed: string | undefined
} {
  const info: AssetInfo = tokenAssetInfo(tokenId)
  return {
    logoURI: resolveTrustedTokenLogoUrl(getTokenLogoURI(info)),
    addressForBlockie: getAddressForBlockie(info),
    blockieSeed: 'token' in info ? undefined : tokenId,
  }
}

function TokenLabel({ tokenId }: { tokenId: string }) {
  const info = tokenAssetInfo(tokenId)
  const { displayLabel } = useTokenDisplayInfo(info)
  return <>{displayLabel}</>
}

/**
 * Searchable token combobox for Swap (GitLab #481).
 * Client-side filter only — token universe is the `tokens` prop (factory graph).
 * Mint and other small lists keep {@link TokenSelect}.
 *
 * Layout stability (GitLab #498): keep the leading logo + reserved padding while open,
 * and keep the selected label visible until the user edits (no empty flash / CLS).
 *
 * Coarse/narrow (GitLab #632 B2): the trigger is a button so browse does not open
 * the IME; search lives inside the portaled menu.
 */
export function TokenSearchSelect({
  value,
  tokens,
  onChange,
  excludeToken,
  'aria-label': ariaLabel,
  disabled,
  loadingLabel = 'Loading tokens...',
  placeholder = 'Search tokens…',
  className,
  id,
}: TokenSearchSelectProps) {
  const rootRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const dropdownRef = useRef<HTMLDivElement>(null)
  const optionTapRectRef = useRef<DOMRect | null>(null)
  const listId = useId()
  const browseWithoutIme = useCoarseNarrowViewport()

  const [open, setOpen] = useState(false)
  /** null = not editing yet (show selected label even while open — avoids empty flash, GitLab #498). */
  const [queryDraft, setQueryDraft] = useState<string | null>(null)
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)
  const activeIndexRef = useRef(0)

  const allowedSet = useMemo(() => new Set(tokens.filter((t) => t !== excludeToken)), [tokens, excludeToken])

  const selectedInfo = useMemo(() => (value ? tokenAssetInfo(value) : null), [value])
  const { displayLabel: selectedLabel } = useTokenDisplayInfo(selectedInfo)

  const searchText = queryDraft ?? ''

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(normalizeTokenSearchQuery(searchText)), TOKEN_SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [searchText])

  useEffect(() => {
    activeIndexRef.current = activeIndex
  }, [activeIndex])

  const queryReady = isTokenSearchQueryReady(debouncedSearch)
  // Closed, or open before first keystroke: keep the selected symbol visible (GitLab #498).
  const inputValue = queryDraft !== null ? queryDraft : selectedLabel

  const options = useMemo(() => {
    if (tokens.length === 0) return [] as string[]

    let result = filterTokensByLocalSearch(tokens, debouncedSearch, { excludeToken })

    // Empty / not-ready query: keep current token at index 0 so Enter re-selects it (#350 / #481).
    // Typed ready query: omit prepend so Enter commits the first search hit.
    if ((!debouncedSearch || !queryReady) && value && allowedSet.has(value) && !result.includes(value)) {
      result = [value, ...result.filter((t) => t !== value)]
    }

    return result
  }, [tokens, debouncedSearch, excludeToken, value, allowedSet, queryReady])

  const canOpen = tokens.length > 0 && !disabled
  const selectedIndex = useMemo(() => options.findIndex((t) => t === value), [options, value])

  useEffect(() => {
    if (!open) return
    if (debouncedSearch && queryReady) {
      setActiveIndex(0)
    } else {
      setActiveIndex(selectedIndex >= 0 ? selectedIndex : 0)
    }
  }, [open, selectedIndex, options.length, debouncedSearch, queryReady])

  const close = useCallback(() => {
    setOpen(false)
    setQueryDraft(null)
    setDebouncedSearch('')
  }, [])

  const selectIndex = useCallback(
    (index: number) => {
      const tokenId = options[index]
      // Selection injection guard: only emit ids present in the factory-gated options list.
      if (!tokenId || !allowedSet.has(tokenId)) return
      onChange(tokenId)
      close()
      inputRef.current?.blur()
    },
    [options, allowedSet, onChange, close]
  )

  const dropdownStyle = usePortalListbox({
    open,
    canShow: canOpen && open,
    anchorRef: rootRef,
    dropdownRef,
    onClose: close,
    preferredMaxHeight: browseWithoutIme ? 320 : 240,
  })

  const handleInputKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement | HTMLButtonElement>) => {
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

  const closeIfFocusLeft = useCallback(() => {
    window.setTimeout(() => {
      const ae = document.activeElement
      if (rootRef.current?.contains(ae)) return
      if (dropdownRef.current?.contains(ae)) return
      close()
    }, 150)
  }, [close])

  const showEmptyState = open && options.length === 0 && queryReady && debouncedSearch.length > 0

  const activeOptionId = open && options.length > 0 ? portalListboxOptionId(listId, activeIndex) : undefined
  const selectedLogo = value ? logoPropsForToken(value) : null

  const rootClassName = className ?? 'relative w-full sm:w-auto sm:min-w-[170px] sm:max-w-[220px] sm:shrink-0'
  const triggerClass = `token-select-trigger w-full${selectedLogo ? ' token-select-trigger--with-leading-logo' : ''}`

  const leadingLogo = selectedLogo ? (
    <span
      className={`token-select-leading-logo pointer-events-none absolute left-3 top-1/2 z-[1] -translate-y-1/2${
        open && queryDraft !== null && !browseWithoutIme ? ' opacity-50' : ''
      }`}
      aria-hidden
      data-testid="token-search-leading-logo"
    >
      <TokenLogo
        size={22}
        logoURI={selectedLogo.logoURI}
        addressForBlockie={selectedLogo.addressForBlockie}
        blockieSeed={selectedLogo.blockieSeed}
      />
    </span>
  ) : null

  return (
    <div ref={rootRef} className={`token-select-root ${rootClassName}`}>
      <div className="relative w-full">
        {/* Keep logo + padding while open so open/close does not shift trigger text (GitLab #498). */}
        {leadingLogo}
        {browseWithoutIme ? (
          <button
            type="button"
            id={id}
            role="combobox"
            className={triggerClass}
            aria-label={ariaLabel}
            aria-autocomplete="list"
            aria-expanded={open}
            aria-controls={listId}
            aria-activedescendant={activeOptionId}
            disabled={!canOpen}
            onClick={() => {
              if (!canOpen) return
              setOpen(true)
            }}
            onKeyDown={handleInputKeyDown}
            onBlur={closeIfFocusLeft}
          >
            <span className="truncate">{canOpen ? selectedLabel || placeholder : loadingLabel}</span>
          </button>
        ) : (
          <input
            ref={inputRef}
            type="text"
            id={id}
            role="combobox"
            className={triggerClass}
            aria-label={ariaLabel}
            aria-autocomplete="list"
            aria-expanded={open}
            aria-controls={listId}
            aria-activedescendant={activeOptionId}
            disabled={!canOpen}
            placeholder={canOpen ? placeholder : loadingLabel}
            value={inputValue}
            maxLength={TOKEN_SEARCH_MAX_QUERY_LENGTH}
            onChange={(e) => {
              setQueryDraft(e.target.value.slice(0, TOKEN_SEARCH_MAX_QUERY_LENGTH))
              if (!open) setOpen(true)
            }}
            onFocus={() => {
              if (!canOpen) return
              setOpen(true)
              // Select label so the next keystroke replaces it without an empty-content flash (#498).
              requestAnimationFrame(() => {
                inputRef.current?.select()
              })
            }}
            onClick={() => {
              if (!canOpen) return
              // Click on an already-focused closed combobox does not fire onFocus (Playwright E7/E8).
              setOpen(true)
            }}
            onKeyDown={handleInputKeyDown}
            onBlur={closeIfFocusLeft}
          />
        )}
        <span
          className="token-select-chevron pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 shrink-0"
          aria-hidden
        />
      </div>

      {open &&
        canOpen &&
        dropdownStyle &&
        createPortal(
          <div
            ref={dropdownRef}
            className="token-select-dropdown token-select-dropdown--menu"
            style={dropdownStyle}
            data-testid="token-select-menu"
          >
            {browseWithoutIme ? (
              <SearchSelectMenuSearch
                inputRef={inputRef}
                value={searchText}
                placeholder={placeholder}
                aria-label={`${ariaLabel} search`}
                onChange={(next) => setQueryDraft(next)}
                onKeyDown={handleInputKeyDown}
              />
            ) : null}
            <ul id={listId} role="listbox" tabIndex={-1} className="token-select-dropdown-list" aria-label={ariaLabel}>
              {showEmptyState ? (
                <li className="px-3 py-2 text-sm" style={{ color: 'var(--ink-dim)' }} role="presentation">
                  No tokens match your search
                </li>
              ) : null}
              {options.map((tokenId, index) => {
                const lp = logoPropsForToken(tokenId)
                const isSelected = tokenId === value
                const isActive = index === activeIndex
                return (
                  <li key={tokenId} role="none">
                    <button
                      type="button"
                      id={portalListboxOptionId(listId, index)}
                      role="option"
                      data-testid={`token-option-${tokenId}`}
                      aria-selected={isSelected}
                      className={`token-select-option w-full flex items-center gap-2${isSelected ? ' token-select-option-active' : ''}${isActive ? ' token-select-option-keyboard-active' : ''}`}
                      onPointerEnter={() => setActiveIndex(index)}
                      onPointerDown={(e) => {
                        optionTapRectRef.current = e.currentTarget.getBoundingClientRect()
                      }}
                      onMouseDown={(e) => e.preventDefault()}
                      onClick={(e) => {
                        const start = optionTapRectRef.current
                        if (start && optionMovedBeyondThreshold(start, e.currentTarget.getBoundingClientRect())) {
                          return
                        }
                        selectIndex(index)
                      }}
                    >
                      <TokenLogo
                        size={22}
                        logoURI={lp.logoURI}
                        addressForBlockie={lp.addressForBlockie}
                        blockieSeed={lp.blockieSeed}
                      />
                      <span className="truncate">
                        <TokenLabel tokenId={tokenId} />
                      </span>
                    </button>
                  </li>
                )
              })}
            </ul>
          </div>,
          document.body
        )}
    </div>
  )
}
