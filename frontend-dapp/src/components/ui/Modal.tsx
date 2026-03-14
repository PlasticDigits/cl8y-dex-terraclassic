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
        className="absolute inset-0 bg-gradient-to-br from-black/75 via-black/70 to-amber-950/30 backdrop-blur-md"
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
        className="relative z-10 w-full max-w-sm border-2 rounded-none overflow-hidden shadow-[6px_6px_0_#000] animate-fade-in-up"
        style={{
          background: 'var(--panel-bg-strong)',
          borderColor: 'rgba(255,255,255,0.4)',
        }}
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? 'modal-title' : undefined}
      >
        {title && (
          <div
            className="flex items-center justify-between px-6 py-4 border-b-2"
            style={{ borderColor: 'rgba(255,255,255,0.2)' }}
          >
            <h2 id="modal-title" className="text-lg font-bold" style={{ color: 'var(--ink)' }}>
              {title}
            </h2>
            <button
              onClick={() => {
                sounds.playButtonPress()
                onClose()
              }}
              className="p-1 transition-colors"
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
