import { useCallback, useMemo, useRef, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ToastContext, type ToastItem } from './toastContextState'

const TOAST_AUTO_DISMISS_MS = 6000

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([])
  const timersRef = useRef<Map<string, number>>(new Map())

  const dismiss = useCallback((id: string) => {
    const timer = timersRef.current.get(id)
    if (timer != null) window.clearTimeout(timer)
    timersRef.current.delete(id)
    setToasts((current) => current.filter((t) => t.id !== id))
  }, [])

  const pushToast = useCallback(
    (kind: ToastItem['kind'], message: string) => {
      const trimmed = message.trim()
      if (!trimmed) return
      const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
      setToasts((current) => [...current.slice(-4), { id, kind, message: trimmed }])
      const timer = window.setTimeout(() => dismiss(id), TOAST_AUTO_DISMISS_MS)
      timersRef.current.set(id, timer)
    },
    [dismiss]
  )

  const value = useMemo(() => ({ pushToast }), [pushToast])

  return (
    <ToastContext.Provider value={value}>
      {children}
      {createPortal(
        <div
          className="fixed top-4 right-4 z-[100] flex w-[min(100vw-2rem,22rem)] flex-col gap-2 pointer-events-none"
          aria-live="polite"
          aria-relevant="additions"
          data-testid="toast-viewport"
        >
          {toasts.map((toast) => (
            <div
              key={toast.id}
              role={toast.kind === 'error' ? 'alert' : 'status'}
              className={`pointer-events-auto rounded-xl border px-3 py-2.5 text-sm shadow-lg ${
                toast.kind === 'success' ? 'alert-success' : 'alert-error'
              }`}
              data-testid={`toast-${toast.kind}`}
            >
              <div className="flex items-start justify-between gap-2">
                <p className="text-left leading-snug">{toast.message}</p>
                <button
                  type="button"
                  className="shrink-0 text-xs opacity-70 hover:opacity-100"
                  aria-label="Dismiss notification"
                  onClick={() => dismiss(toast.id)}
                >
                  ×
                </button>
              </div>
            </div>
          ))}
        </div>,
        document.body
      )}
    </ToastContext.Provider>
  )
}
