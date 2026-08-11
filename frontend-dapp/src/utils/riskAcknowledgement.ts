/** Bump when disclaimer copy materially changes so returning users re-acknowledge. */
export const RISK_ACK_VERSION = 1

export const RISK_ACK_STORAGE_KEY = 'cl8y-dex-risk-ack'

type StoredAck = { v: number }

function parseStored(raw: string | null): StoredAck | null {
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object') return null
    const v = (parsed as { v?: unknown }).v
    if (typeof v !== 'number' || !Number.isFinite(v)) return null
    return { v }
  } catch {
    return null
  }
}

export function readRiskAcknowledgement(): StoredAck | null {
  if (typeof window === 'undefined') return null
  return parseStored(window.localStorage.getItem(RISK_ACK_STORAGE_KEY))
}

export function hasRiskAcknowledgement(): boolean {
  const s = readRiskAcknowledgement()
  return s !== null && s.v >= RISK_ACK_VERSION
}

export function setRiskAcknowledged(): void {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(RISK_ACK_STORAGE_KEY, JSON.stringify({ v: RISK_ACK_VERSION }))
}

/**
 * Playwright webServer sets this so E2E is not blocked by the first-visit modal (GitLab #138).
 * Same flag also skips the connected Legal clickwrap gate (GitLab #517).
 */
export function skipRiskAcknowledgementForAutomation(): boolean {
  return import.meta.env.VITE_PLAYWRIGHT_E2E === 'true'
}
