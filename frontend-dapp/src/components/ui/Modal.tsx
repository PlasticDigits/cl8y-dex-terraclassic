import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { sounds } from '@/lib/sounds'

export interface ModalProps {
  isOpen: boolean
  onClose: () => void
  title?: string
  children: React.ReactNode
}

export function Modal({ isOpen, onClose, title, children }: ModalProps) {
  const modalRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!isOpen) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [isOpen, onClose])

  useEffect(() => {
    if (isOpen) {
      modalRef.current?.focus()
    }
  }, [isOpen])

  if (!isOpen) return null

  return createPortal(
    <div className="fixed inset-0 z-[9999] flex items-center justify-center p-4">
      <div
        className="app-modal-backdrop"
        onClick={() => {
          sounds.playButtonPress()
          onClose()
        }}
        role="presentation"
        aria-hidden="true"
      />
      <div
        ref={modalRef}
        tabIndex={-1}
        className="app-modal-panel animate-fade-in-up"
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? 'modal-title' : undefined}
      >
        {title && (
          <div className="app-modal-header">
            <h2 id="modal-title" className="text-lg font-semibold font-heading" style={{ color: 'var(--ink)' }}>
              {title}
            </h2>
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
          </div>
        )}
        {children}
      </div>
    </div>,
    document.body
  )
}
