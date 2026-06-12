import type { TerraExecuteContractEntry } from './terraBroadcast'

function readDeadline(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value) && value > 0) return Math.floor(value)
  if (typeof value === 'string' && /^\d+$/.test(value)) return Number.parseInt(value, 10)
  return null
}

function scanMsgForDeadline(msg: Record<string, unknown>): number | null {
  let best: number | null = null

  const direct = readDeadline(msg.deadline)
  if (direct != null) best = direct

  const send = msg.send
  if (send && typeof send === 'object') {
    const hook = (send as Record<string, unknown>).msg
    if (hook && typeof hook === 'object') {
      const nested = scanMsgForDeadline(hook as Record<string, unknown>)
      if (nested != null && (best == null || nested > best)) best = nested
    }
  }

  const swap = msg.swap
  if (swap && typeof swap === 'object') {
    const nested = scanMsgForDeadline(swap as Record<string, unknown>)
    if (nested != null && (best == null || nested > best)) best = nested
  }

  return best
}

/** Latest on-chain msg deadline across execute entries, if any. */
export function terraRecoveryDeadlineUnixFromEntries(entries: TerraExecuteContractEntry[]): number | null {
  let best: number | null = null
  for (const entry of entries) {
    const d = scanMsgForDeadline(entry.msg)
    if (d != null && (best == null || d > best)) best = d
  }
  return best
}

/** Poll through msg deadline or a retail fallback window (dex default 300s). */
export function terraRecoveryPollDeadlineUnix(entries: TerraExecuteContractEntry[], fallbackSeconds = 300): number {
  const now = Math.floor(Date.now() / 1000)
  const fromMsg = terraRecoveryDeadlineUnixFromEntries(entries)
  if (fromMsg != null && fromMsg > now) return fromMsg
  return now + fallbackSeconds
}
