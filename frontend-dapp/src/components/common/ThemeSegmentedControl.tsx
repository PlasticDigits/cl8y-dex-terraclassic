export type ThemeMode = 'dark' | 'light'

type ThemeSegmentedControlProps = {
  theme: ThemeMode
  onSelect: (mode: ThemeMode) => void
  groupClassName: string
  /** Footer uses short labels; mobile sheet uses longer labels for clarity. */
  labelStyle: 'short' | 'long'
}

export function ThemeSegmentedControl({ theme, onSelect, groupClassName, labelStyle }: ThemeSegmentedControlProps) {
  const darkLabel = labelStyle === 'short' ? 'Dark' : 'Dark theme'
  const lightLabel = labelStyle === 'short' ? 'Light' : 'Light theme'

  return (
    <div className={groupClassName} role="group" aria-label="Theme">
      <button
        type="button"
        aria-pressed={theme === 'dark'}
        className={`app-footer-theme-button${theme === 'dark' ? ' app-footer-theme-button-active' : ''}`}
        onClick={() => onSelect('dark')}
      >
        {darkLabel}
      </button>
      <button
        type="button"
        aria-pressed={theme === 'light'}
        className={`app-footer-theme-button${theme === 'light' ? ' app-footer-theme-button-active' : ''}`}
        onClick={() => onSelect('light')}
      >
        {lightLabel}
      </button>
    </div>
  )
}
