import { useCallback, useEffect, useId, useRef, useState } from 'react'
import { sounds } from '@/lib/sounds'
import { COPY_BUTTON_FAILURE_MESSAGE, COPY_BUTTON_SUCCESS_MESSAGE } from '@/utils/copyButtonCopy'
import { copyToClipboard } from '@/utils/copyToClipboard'

const SUCCESS_RESET_MS = 2000

export type CopyButtonProps = {
  /** Full string written to the clipboard (e.g. bech32 address, tx hash). */
  text: string
  /** Accessible name for the control (e.g. "Copy wallet address"). */
  ariaLabel: string
  className?: string
  /** Override default success phrase in the aria-live region. */
  successMessage?: string
  /** When set, renders as a full-width wallet dropdown row with visible label (GitLab #185). */
  menuLabel?: string
  /** Visible label on a normal button (not a menu item) — WalletConnect pairing copy (#519). */
  buttonLabel?: string
  'data-testid'?: string
}

type Feedback = 'idle' | 'success' | 'error'

const copyIcon = (
  <svg className="h-4 w-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
    />
  </svg>
)

export function CopyButton({
  text,
  ariaLabel,
  className = '',
  successMessage = COPY_BUTTON_SUCCESS_MESSAGE,
  menuLabel,
  buttonLabel,
  'data-testid': testId = 'copy-button',
}: CopyButtonProps) {
  const liveId = useId()
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [feedback, setFeedback] = useState<Feedback>('idle')

  const clearResetTimer = useCallback(() => {
    if (resetTimer.current) {
      clearTimeout(resetTimer.current)
      resetTimer.current = null
    }
  }, [])

  useEffect(() => clearResetTimer, [clearResetTimer])

  const scheduleIdle = useCallback(() => {
    clearResetTimer()
    resetTimer.current = setTimeout(() => setFeedback('idle'), SUCCESS_RESET_MS)
  }, [clearResetTimer])

  const handleClick = useCallback(async () => {
    sounds.playButtonPress()
    clearResetTimer()
    const result = await copyToClipboard(text)
    if (result.ok) {
      setFeedback('success')
      scheduleIdle()
      return
    }
    setFeedback('error')
    scheduleIdle()
  }, [text, clearResetTimer, scheduleIdle])

  const liveMessage = feedback === 'success' ? successMessage : feedback === 'error' ? COPY_BUTTON_FAILURE_MESSAGE : ''

  if (buttonLabel) {
    return (
      <button
        type="button"
        className={`btn-muted walletconnect-pairing-copy ${className}`.trim()}
        style={{ color: 'var(--ink)' }}
        aria-label={ariaLabel}
        aria-describedby={liveMessage ? liveId : undefined}
        data-testid={testId}
        onClick={() => void handleClick()}
      >
        {copyIcon}
        {buttonLabel}
        <span id={liveId} className="sr-only" aria-live="polite" aria-atomic="true">
          {liveMessage}
        </span>
      </button>
    )
  }

  if (menuLabel) {
    return (
      <button
        type="button"
        role="menuitem"
        className={`wallet-menu-item ${className}`.trim()}
        style={{ color: 'var(--ink-dim)' }}
        aria-label={ariaLabel}
        aria-describedby={liveMessage ? liveId : undefined}
        data-testid={testId}
        onClick={() => void handleClick()}
      >
        {copyIcon}
        {menuLabel}
        <span id={liveId} className="sr-only" aria-live="polite" aria-atomic="true">
          {liveMessage}
        </span>
      </button>
    )
  }

  return (
    <span className={`inline-flex items-center gap-1 ${className}`.trim()}>
      <button
        type="button"
        className="copy-button"
        aria-label={ariaLabel}
        data-testid={testId}
        onClick={() => void handleClick()}
      >
        {copyIcon}
      </button>
      <span id={liveId} className="sr-only" aria-live="polite" aria-atomic="true">
        {liveMessage}
      </span>
    </span>
  )
}
