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
