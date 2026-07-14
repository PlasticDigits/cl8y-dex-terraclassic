import { MoonIcon, SunIcon } from '@/components/common/shellPrefIcons'

export type ThemeMode = 'dark' | 'light'

type ThemeSegmentedControlProps = {
  theme: ThemeMode
  onSelect: (mode: ThemeMode) => void
  groupClassName: string
  /**
   * Kept for Layout API compatibility. Visible chrome is icon-only;
   * aria-labels stay explicit for assistive tech.
   */
  labelStyle: 'short' | 'long'
}

export function ThemeSegmentedControl({ theme, onSelect, groupClassName }: ThemeSegmentedControlProps) {
  return (
    <div className={groupClassName} role="group" aria-label="Theme">
      <button
        type="button"
        aria-label="Dark theme"
        title="Dark theme"
        aria-pressed={theme === 'dark'}
        className={`app-footer-theme-button app-pref-icon-button${theme === 'dark' ? ' app-footer-theme-button-active' : ''}`}
        onClick={() => onSelect('dark')}
      >
        <MoonIcon />
      </button>
      <button
        type="button"
        aria-label="Light theme"
        title="Light theme"
        aria-pressed={theme === 'light'}
        className={`app-footer-theme-button app-pref-icon-button${theme === 'light' ? ' app-footer-theme-button-active' : ''}`}
        onClick={() => onSelect('light')}
      >
        <SunIcon />
      </button>
    </div>
  )
}
