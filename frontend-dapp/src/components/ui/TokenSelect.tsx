import { useCallback, useId, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { usePortalListbox } from './PortalListbox'
import { usePortalListboxKeyboard } from './usePortalListboxKeyboard'
import type { AssetInfo } from '@/types'
import { tokenAssetInfo } from '@/types'
import { getAddressForBlockie, getCachedTokenSymbol, getTokenLogoURI } from '@/utils/tokenDisplay'
import { resolveTrustedTokenLogoUrl } from '@/utils/tokenLogoAllowlist'
import { useTokenDisplayInfo } from '@/hooks/useTokenDisplayInfo'
import { TokenLogo } from './TokenLogo'

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

function tokenTypeaheadLabel(tokenId: string): string {
  return getCachedTokenSymbol(tokenId) ?? ''
}

export interface TokenSelectProps {
  value: string
  tokens: string[]
  onChange: (tokenId: string) => void
  excludeToken?: string
  'aria-label': string
  disabled?: boolean
  loadingLabel?: string
}

function TokenLabel({ tokenId }: { tokenId: string }) {
  const info = tokenAssetInfo(tokenId)
  const { displayLabel } = useTokenDisplayInfo(info)
  return <>{displayLabel}</>
}

/** Button-trigger token listbox (Mint faucet). Swap uses TokenSearchSelect (#481). */
export function TokenSelect({
  value,
  tokens,
  onChange,
  excludeToken,
  'aria-label': ariaLabel,
  disabled,
  loadingLabel = 'Loading tokens...',
}: TokenSelectProps) {
  const [open, setOpen] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const dropdownRef = useRef<HTMLUListElement>(null)
  const listId = useId()

  const options = tokens.filter((t) => t !== excludeToken)
  const triggerLabel = options.length === 0 ? loadingLabel : value ? <TokenLabel tokenId={value} /> : 'Select token'

  const close = useCallback(() => setOpen(false), [])
  const openMenu = useCallback(() => setOpen(true), [])

  const selectedIndex = useMemo(() => options.findIndex((t) => t === value), [options, value])

  const getTypeaheadLabel = useCallback((index: number) => tokenTypeaheadLabel(options[index] ?? ''), [options])

  const { activeOptionId, getOptionClassName, getOptionId, handleTriggerKeyDown, primeActiveIndexForClickOpen } =
    usePortalListboxKeyboard({
      open,
      canOpen: options.length > 0 && !disabled,
      optionCount: options.length,
      selectedIndex,
      getTypeaheadLabel,
      listId,
      triggerRef,
      listboxRef: dropdownRef,
      onOpen: openMenu,
      onClose: close,
      onSelectIndex: (index) => {
        const tokenId = options[index]!
        if (tokenId !== value) onChange(tokenId)
      },
    })

  const dropdownStyle = usePortalListbox({
    open,
    canShow: options.length > 0,
    anchorRef: rootRef,
    dropdownRef,
    onClose: close,
    preferredMaxHeight: 240,
  })

  const selectedLogo = value ? logoPropsForToken(value) : null

  return (
    <div
      ref={rootRef}
      className="token-select-root relative w-full sm:w-auto sm:min-w-[170px] sm:max-w-[220px] sm:shrink-0"
    >
      <button
        ref={triggerRef}
        type="button"
        disabled={disabled || options.length === 0}
        className="token-select-trigger"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onKeyDown={handleTriggerKeyDown}
        onClick={() => {
          if (options.length === 0) return
          setOpen((o) => {
            if (!o) primeActiveIndexForClickOpen()
            return !o
          })
        }}
      >
        {selectedLogo && (
          <TokenLogo
            size={22}
            logoURI={selectedLogo.logoURI}
            addressForBlockie={selectedLogo.addressForBlockie}
            blockieSeed={selectedLogo.blockieSeed}
          />
        )}
        <span className="truncate flex-1 text-left">{triggerLabel}</span>
        <span className="token-select-chevron shrink-0" aria-hidden />
      </button>

      {open &&
        options.length > 0 &&
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
            {options.map((tokenId, index) => {
              const lp = logoPropsForToken(tokenId)
              const isSelected = tokenId === value
              return (
                <li key={tokenId} role="none">
                  <button
                    type="button"
                    id={getOptionId(index)}
                    role="option"
                    data-testid={`token-option-${tokenId}`}
                    aria-selected={isSelected}
                    className={getOptionClassName(index, isSelected)}
                    onClick={() => {
                      onChange(tokenId)
                      close()
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
          </ul>,
          document.body
        )}
    </div>
  )
}
