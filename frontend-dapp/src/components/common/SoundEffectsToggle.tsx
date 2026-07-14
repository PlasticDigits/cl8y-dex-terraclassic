type SoundEffectsToggleProps = {
  enabled: boolean
  onToggle: (enabled: boolean) => void
  /** Header uses short label; mobile More sheet uses a longer label. */
  labelStyle: 'short' | 'long'
  className?: string
}

/**
 * Shell control for UI SFX mute (GitLab #487).
 * `aria-pressed` reflects whether sounds are **enabled** (pressed = on).
 */
export function SoundEffectsToggle({ enabled, onToggle, labelStyle, className = '' }: SoundEffectsToggleProps) {
  const label = labelStyle === 'short' ? (enabled ? 'Sound' : 'Muted') : enabled ? 'Sound on' : 'Sound off'

  return (
    <button
      type="button"
      className={`app-footer-theme-button${enabled ? ' app-footer-theme-button-active' : ''}${className ? ` ${className}` : ''}`}
      aria-pressed={enabled}
      aria-label={enabled ? 'Mute sound effects' : 'Enable sound effects'}
      title={enabled ? 'Mute sound effects' : 'Enable sound effects'}
      onClick={() => onToggle(!enabled)}
    >
      {label}
    </button>
  )
}
