export const RELATIVE_AGE_FALLBACK = '—'

const MINUTE_MS = 60_000
const HOUR_MS = 60 * MINUTE_MS
const DAY_MS = 24 * HOUR_MS
const YEAR_MS = 365 * DAY_MS
/** Clock skew: timestamps this far in the future still format as `just now`. */
const FUTURE_CLAMP_MS = 2 * MINUTE_MS
const ISO_CREATED_AT_MAX_LEN = 64
/** RFC3339 / ISO-8601 with 4-digit year. Rejects unix-ms, HTML, and year-0 padded forms we still check. */
const ISO_CREATED_AT = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/

export function formatTime(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

/** Terra oracle timestamps are Unix seconds. */
export function formatTimeFromUnixSeconds(sec: number): string {
  if (sec <= 0) return '—'
  return formatTime(new Date(sec * 1000).toISOString())
}

export function formatDateTime(iso: string | null): string {
  if (!iso) return '—'
  const d = new Date(iso)
  return d.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

function unitPhrase(n: number, unit: string): string {
  return n === 1 ? `1 ${unit} ago` : `${n} ${unit}s ago`
}

/**
 * Parse indexer `created_at` for `/pool` age (GitLab #662).
 * Accepts ISO-8601 / RFC3339 only. Returns epoch ms or null.
 */
export function parseCreatedAtMs(iso: string | null | undefined): number | null {
  if (typeof iso !== 'string') return null
  const trimmed = iso.trim()
  if (!trimmed || trimmed.length > ISO_CREATED_AT_MAX_LEN) return null
  const m = ISO_CREATED_AT.exec(trimmed)
  if (!m) return null
  const year = Number(m[1])
  if (!Number.isFinite(year) || year < 1) return null
  const t = Date.parse(trimmed)
  if (!Number.isFinite(t)) return null
  return t
}

/**
 * Retail relative age for `/pool` Created cells.
 * Inject `nowMs` in tests. Missing / garbage / far-future → `—`.
 */
export function formatRelativeAge(iso: string | null | undefined, nowMs?: number): string {
  const t = parseCreatedAtMs(iso)
  if (t == null) return RELATIVE_AGE_FALLBACK
  const now = nowMs ?? Date.now()
  if (!Number.isFinite(now)) return RELATIVE_AGE_FALLBACK
  const delta = now - t
  if (delta < -FUTURE_CLAMP_MS) return RELATIVE_AGE_FALLBACK
  if (delta < MINUTE_MS) return 'just now'
  const minutes = Math.floor(delta / MINUTE_MS)
  if (minutes < 60) return unitPhrase(minutes, 'minute')
  const hours = Math.floor(delta / HOUR_MS)
  if (hours < 24) return unitPhrase(hours, 'hour')
  const days = Math.floor(delta / DAY_MS)
  if (days < 365) return unitPhrase(days, 'day')
  return unitPhrase(Math.floor(delta / YEAR_MS), 'year')
}

/** Hover title for a valid `created_at`; omit when the cell would show `—`. Never the raw payload. */
export function formatCreatedAtTitle(iso: string | null | undefined): string | undefined {
  const t = parseCreatedAtMs(iso)
  if (t == null) return undefined
  return formatDateTime(new Date(t).toISOString())
}
