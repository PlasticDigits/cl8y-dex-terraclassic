import { useSyncExternalStore } from 'react'

export type NumberDisplayMode = 'compact' | 'full'
export const NUMBER_DISPLAY_MODE_KEY = 'cl8y-dex-number-display-mode'

/** Storage is the only persistence source; URL/query preferences are never read. */
export function readNumberDisplayMode(): NumberDisplayMode {
  if (typeof window === 'undefined') return 'compact'
  try {
    return window.localStorage.getItem(NUMBER_DISPLAY_MODE_KEY) === 'full' ? 'full' : 'compact'
  } catch {
    return 'compact'
  }
}

const listeners = new Set<() => void>()

export function writeNumberDisplayMode(mode: NumberDisplayMode): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(NUMBER_DISPLAY_MODE_KEY, mode === 'full' ? 'full' : 'compact')
  } catch {
    // Quota/private-mode failures leave the persisted preference unchanged.
  }
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  const onStorage = (event: StorageEvent) => {
    if (event.key === NUMBER_DISPLAY_MODE_KEY || event.key === null) listener()
  }
  window.addEventListener('storage', onStorage)
  return () => {
    listeners.delete(listener)
    window.removeEventListener('storage', onStorage)
  }
}

/** Subscribe at rendering boundaries; changing display never invalidates queries or remounts inputs. */
export function useNumberDisplayMode(): readonly [NumberDisplayMode, typeof writeNumberDisplayMode] {
  const mode = useSyncExternalStore(subscribe, readNumberDisplayMode, () => 'compact' as const)
  return [mode, writeNumberDisplayMode] as const
}
