/**
 * Format faucet cooldown seconds for display — `mm:ss` under one hour, otherwise a short human string.
 */
export function formatFaucetCooldown(secondsRemaining: number): string {
  const secs = Math.max(0, Math.floor(secondsRemaining))
  if (secs === 0) return '0:00'

  if (secs >= 3600) {
    const hours = Math.floor(secs / 3600)
    const minutes = Math.floor((secs % 3600) / 60)
    if (minutes === 0) return `${hours}h`
    return `${hours}h ${minutes}m`
  }

  const minutes = Math.floor(secs / 60)
  const seconds = secs % 60
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}
