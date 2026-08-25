import { TOKEN_SEARCH_MAX_QUERY_LENGTH } from '@/utils/tokenSearchQuery'

type SearchSelectMenuSearchProps = {
  inputRef: React.RefObject<HTMLInputElement | null>
  value: string
  placeholder: string
  'aria-label': string
  maxLength?: number
  onChange: (value: string) => void
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void
}

/**
 * Explicit search field inside a portaled picker (GitLab #632 B2).
 * Lives in the menu so opening the trigger does not focus a text field / IME.
 */
export function SearchSelectMenuSearch({
  inputRef,
  value,
  placeholder,
  'aria-label': ariaLabel,
  maxLength = TOKEN_SEARCH_MAX_QUERY_LENGTH,
  onChange,
  onKeyDown,
}: SearchSelectMenuSearchProps) {
  return (
    <div className="token-select-dropdown-search-wrap">
      <input
        ref={inputRef}
        type="search"
        className="token-select-dropdown-search"
        aria-label={ariaLabel}
        placeholder={placeholder}
        value={value}
        maxLength={maxLength}
        autoComplete="off"
        autoCorrect="off"
        spellCheck={false}
        onChange={(e) => onChange(e.target.value.slice(0, maxLength))}
        onKeyDown={onKeyDown}
      />
    </div>
  )
}
