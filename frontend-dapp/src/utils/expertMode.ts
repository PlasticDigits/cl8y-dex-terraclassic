/** Bump when expert-mode disclaimer copy materially changes. */
export const EXPERT_MODE_ACK_VERSION = 1

/** Typed confirmation phrase for enabling expert mode (GitLab #378 / M-15). */
export const EXPERT_MODE_CONFIRM_PHRASE = 'enable expert mode'

export const EXPERT_MODE_STORAGE_KEY = 'cl8y-dex-expert-mode'

type StoredExpertMode = { v: number; enabled: boolean }

function parseStored(raw: string | null): StoredExpertMode | null {
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') return null
    const v = (parsed as { v?: unknown }).v
    const enabled = (parsed as { enabled?: unknown }).enabled
    if (typeof v !== 'number' || !Number.isFinite(v)) return null
    if (typeof enabled !== 'boolean') return null
    return { v, enabled }
  } catch {
    return null
  }
}

export function readExpertMode(): boolean {
  if (typeof window === 'undefined') return false
  const stored = parseStored(window.localStorage.getItem(EXPERT_MODE_STORAGE_KEY))
  if (!stored || stored.v < EXPERT_MODE_ACK_VERSION) return false
  return stored.enabled
}

export function writeExpertMode(enabled: boolean): void {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(EXPERT_MODE_STORAGE_KEY, JSON.stringify({ v: EXPERT_MODE_ACK_VERSION, enabled }))
}
