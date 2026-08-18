import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { sounds } from '@/lib/sounds'

export interface ModalProps {
  isOpen: boolean
  onClose: () => void
  title?: string
  children: React.ReactNode
  /** Appended to the panel element for layout variants (e.g. wider risk modal). */
  panelClassName?: string
  /**
   * When false, Escape and backdrop clicks do not close the dialog and the header close control is hidden.
   * Use for blocking confirmations (e.g. first-visit risk acknowledgement — GitLab #138).
   */
  dismissible?: boolean
  /**
   * Portal stacking class. WalletConnect pairing must sit above Connect Wallet (GitLab #554).
   * Default `z-[9999]`. Pairing sheet uses `z-[10001]`.
   */
  zIndexClassName?: string
  /** Optional test id on the portal root. */
  rootTestId?: string
}

export function Modal({
  isOpen,
  onClose,
  title,
  children,
  dismissible = true,
  panelClassName,
  zIndexClassName = 'z-[9999]',
  rootTestId,
}: ModalProps) {
  const modalRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!isOpen || !dismissible) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [isOpen, onClose, dismissible])

  useEffect(() => {
    if (isOpen) {
      modalRef.current?.focus()
    }
  }, [isOpen])

  useEffect(() => {
    if (!isOpen || !dismissible) return
    const panel = modalRef.current
    if (!panel) return

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Tab') return
      const focusables = panel.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )
      if (focusables.length === 0) return
      const first = focusables[0]
      const last = focusables[focusables.length - 1]
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault()
        first.focus()
      }
    }

    panel.addEventListener('keydown', onKeyDown)
    return () => panel.removeEventListener('keydown', onKeyDown)
  }, [isOpen, dismissible])

  if (!isOpen) return null

  return createPortal(
    <div
      className={`app-modal-portal-root fixed inset-0 ${zIndexClassName} flex items-center justify-center p-4`}
      data-testid={rootTestId}
    >
      <div
        className="app-modal-backdrop"
        onClick={
          dismissible
            ? () => {
                sounds.playButtonPress()
                onClose()
              }
            : undefined
        }
        role="presentation"
        aria-hidden="true"
      />
      <div
        ref={modalRef}
        tabIndex={-1}
        className={`app-modal-panel animate-fade-in-up${panelClassName ? ` ${panelClassName}` : ''}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? 'modal-title' : undefined}
      >
        {title && (
          <div className={`app-modal-header${dismissible ? '' : ' app-modal-header--blocking'}`}>
            <h2 id="modal-title" className="text-lg font-semibold font-heading" style={{ color: 'var(--ink)' }}>
              {title}
            </h2>
            {dismissible ? (
              <button
                onClick={() => {
                  sounds.playButtonPress()
                  onClose()
                }}
                className="btn-muted !min-h-0 !px-2.5 !py-2"
                style={{ color: 'var(--ink-subtle)' }}
                aria-label="Close modal"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            ) : null}
          </div>
        )}
        {children}
      </div>
    </div>,
    document.body
  )
}
