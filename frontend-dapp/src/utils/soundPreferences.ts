/**
 * Client-local UI sound preference (GitLab #487).
 *
 * Invariants:
 * - Default ON when the key is missing or invalid (preserve historical SFX behavior).
 * - Stored as `'1'` / `'0'` under `cl8y-dex-sounds-enabled` (same pattern as swap settings).
 * - Session cache wins after first read/write so mute applies immediately and survives
 *   localStorage write failures (private mode / quota) for the current tab.
 * - Cross-tab sync via `storage` events is out of scope for MVP.
 */

export const SOUNDS_ENABLED_STORAGE_KEY = 'cl8y-dex-sounds-enabled'

/** In-tab cache; `undefined` means “not loaded from storage yet”. */
let sessionSoundsEnabled: boolean | undefined

function parseStoredSoundsEnabled(raw: string | null): boolean {
  if (raw === '0') return false
  if (raw === '1') return true
  // Missing or garbage → default ON
  return true
}

export function readSoundsEnabled(): boolean {
  if (sessionSoundsEnabled !== undefined) return sessionSoundsEnabled
  if (typeof window === 'undefined') return true
  try {
    sessionSoundsEnabled = parseStoredSoundsEnabled(window.localStorage.getItem(SOUNDS_ENABLED_STORAGE_KEY))
  } catch {
    sessionSoundsEnabled = true
  }
  return sessionSoundsEnabled
}

export function writeSoundsEnabled(enabled: boolean): void {
  sessionSoundsEnabled = enabled
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(SOUNDS_ENABLED_STORAGE_KEY, enabled ? '1' : '0')
  } catch {
    // ignore quota / private mode — session cache still applies
  }
}

/** Test-only: clear the in-tab cache so the next read hits storage again. */
export function resetSoundsEnabledCacheForTests(): void {
  sessionSoundsEnabled = undefined
}
