import { SoundOffIcon, SoundOnIcon } from '@/components/common/shellPrefIcons'

type SoundEffectsToggleProps = {
  enabled: boolean
  onToggle: (enabled: boolean) => void
  /**
   * Kept for Layout API compatibility. Visible chrome is icon-only;
   * aria-labels stay explicit for assistive tech.
   */
  labelStyle: 'short' | 'long'
  className?: string
}

/**
 * Shell control for UI SFX mute (GitLab #487).
 * `aria-pressed` reflects whether sounds are **enabled** (pressed = on).
 */
export function SoundEffectsToggle({ enabled, onToggle, className = '' }: SoundEffectsToggleProps) {
  const actionLabel = enabled ? 'Mute sound effects' : 'Enable sound effects'

  return (
    <button
      type="button"
      className={`app-footer-theme-button app-pref-icon-button${enabled ? ' app-footer-theme-button-active' : ''}${className ? ` ${className}` : ''}`}
      aria-pressed={enabled}
      aria-label={actionLabel}
      title={actionLabel}
      onClick={() => onToggle(!enabled)}
    >
      {enabled ? <SoundOnIcon /> : <SoundOffIcon />}
    </button>
  )
}
