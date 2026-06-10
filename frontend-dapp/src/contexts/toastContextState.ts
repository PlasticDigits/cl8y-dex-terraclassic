import { createContext, useContext } from 'react'
import { humanizeUserFacingError } from '@/utils/humanizeUserFacingError'

export type ToastKind = 'success' | 'error'

export type ToastItem = {
  id: string
  kind: ToastKind
  message: string
}

export type ToastContextValue = {
  pushToast: (kind: ToastKind, message: string) => void
}

export const ToastContext = createContext<ToastContextValue | null>(null)

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext)
  if (!ctx) throw new Error('useToast must be used within ToastProvider')
  return ctx
}

/** Safe when ToastProvider is absent (unit tests). */
export function useOptionalToast(): ToastContextValue | null {
  return useContext(ToastContext)
}

export function toastErrorMessage(error: unknown): string {
  return humanizeUserFacingError(error instanceof Error ? error : new Error(String(error)))
}
