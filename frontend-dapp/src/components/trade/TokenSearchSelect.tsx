import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { usePortalListbox } from '@/components/ui/PortalListbox'
import { movePortalListboxActiveIndex, portalListboxOptionId } from '@/components/ui/portalListboxKeyboard'
import { TokenLogo } from '@/components/ui/TokenLogo'
import type { AssetInfo } from '@/types'
import { tokenAssetInfo } from '@/types'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'
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
  const dropdownRef = useRef<HTMLUListElement>(null)
  const listId = useId()

  const [open, setOpen] = useState(false)
  const [searchText, setSearchText] = useState('')
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)
  const activeIndexRef = useRef(0)

  const allowedSet = useMemo(() => new Set(tokens.filter((t) => t !== excludeToken)), [tokens, excludeToken])

  const selectedInfo = useMemo(() => (value ? tokenAssetInfo(value) : null), [value])
  const { displayLabel: selectedLabel } = useTokenDisplayInfo(selectedInfo)

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(normalizeTokenSearchQuery(searchText)), TOKEN_SEARCH_DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [searchText])

  useEffect(() => {
    activeIndexRef.current = activeIndex
  }, [activeIndex])

  const queryReady = isTokenSearchQueryReady(debouncedSearch)
  const inputValue = open ? searchText : selectedLabel

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
    setSearchText('')
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
    preferredMaxHeight: 240,
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

  const showEmptyState = open && options.length === 0 && queryReady && debouncedSearch.length > 0

  const activeOptionId = open && options.length > 0 ? portalListboxOptionId(listId, activeIndex) : undefined
  const selectedLogo = value ? logoPropsForToken(value) : null

  const rootClassName = className ?? 'relative w-full sm:w-auto sm:min-w-[170px] sm:max-w-[220px] sm:shrink-0'

  return (
    <div ref={rootRef} className={`token-select-root ${rootClassName}`}>
      <div className="relative w-full">
        {!open && selectedLogo ? (
          <span className="pointer-events-none absolute left-3 top-1/2 z-[1] -translate-y-1/2" aria-hidden>
            <TokenLogo
              size={22}
              logoURI={selectedLogo.logoURI}
              addressForBlockie={selectedLogo.addressForBlockie}
              blockieSeed={selectedLogo.blockieSeed}
            />
          </span>
        ) : null}
        <input
          ref={inputRef}
          type="text"
          id={id}
          role="combobox"
          className={`token-select-trigger w-full${!open && selectedLogo ? ' !pl-11' : ''}`}
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
            setSearchText(e.target.value.slice(0, TOKEN_SEARCH_MAX_QUERY_LENGTH))
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
      </div>

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
            style={dropdownStyle}
          >
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
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => selectIndex(index)}
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
          </ul>,
          document.body
        )}
    </div>
  )
}
