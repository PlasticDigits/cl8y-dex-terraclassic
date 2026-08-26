import { useCallback, useEffect, useId, useRef, useState } from 'react'
import { sounds } from '@/lib/sounds'
import { COPY_BUTTON_FAILURE_MESSAGE } from '@/utils/copyButtonCopy'
import { copyToClipboard } from '@/utils/copyToClipboard'
import {
  resolveNavigatorCanShare,
  resolveNavigatorShare,
  shareOrCopyPageLink,
  type CanShareFn,
  type CopyFn,
  type ShareFn,
} from '@/utils/sharePageLink'
import { SHARE_LINK_BUTTON_LABEL, SHARE_LINK_COPIED_MESSAGE } from '@/utils/sharePageLinkCopy'

const SUCCESS_RESET_MS = 2000

export type ShareLinkButtonProps = {
  url: string
  title: string
  text: string
  ariaLabel: string
  buttonLabel?: string
  className?: string
  'data-testid'?: string
  share?: ShareFn
  canShare?: CanShareFn
  copy?: CopyFn
}

type Feedback = 'idle' | 'success' | 'error'

const shareIcon = (
  <svg className="h-4 w-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M4 12v8a2 2 0 002 2h12a2 2 0 002-2v-8M16 6l-4-4-4 4M12 2v13"
    />
  </svg>
)

export function ShareLinkButton({
  url,
  title,
  text,
  ariaLabel,
  buttonLabel = SHARE_LINK_BUTTON_LABEL,
  className = '',
  'data-testid': testId = 'share-link-button',
  share: shareProp,
  canShare: canShareProp,
  copy: copyProp,
}: ShareLinkButtonProps) {
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
    const result = await shareOrCopyPageLink(
      { url, title, text },
      {
        share: shareProp ?? resolveNavigatorShare(),
        canShare: canShareProp ?? resolveNavigatorCanShare(),
        copy: copyProp ?? copyToClipboard,
      }
    )
    if (result.outcome === 'shared' || result.outcome === 'aborted') {
      setFeedback('idle')
      return
    }
    if (result.outcome === 'copied') {
      setFeedback('success')
      scheduleIdle()
      return
    }
    setFeedback('error')
    scheduleIdle()
  }, [url, title, text, shareProp, canShareProp, copyProp, clearResetTimer, scheduleIdle])

  const liveMessage =
    feedback === 'success' ? SHARE_LINK_COPIED_MESSAGE : feedback === 'error' ? COPY_BUTTON_FAILURE_MESSAGE : ''

  return (
    <button
      type="button"
      className={`btn-muted share-link-button ${className}`.trim()}
      style={{ color: 'var(--ink)' }}
      aria-label={ariaLabel}
      aria-describedby={liveMessage ? liveId : undefined}
      data-testid={testId}
      onClick={() => void handleClick()}
    >
      {shareIcon}
      {buttonLabel}
      <span id={liveId} className="sr-only" aria-live="polite" aria-atomic="true">
        {liveMessage}
      </span>
    </button>
  )
}
