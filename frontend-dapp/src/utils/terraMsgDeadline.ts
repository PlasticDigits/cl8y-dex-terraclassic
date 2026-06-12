import type { TerraExecuteContractEntry } from '@/services/terraclassic/terraBroadcast'
import { TERRA_TX_RECOVERY_DEFAULT_SECONDS } from '@/utils/terraTxTimeout'

function deadlineFromDecodedMsg(msg: Record<string, unknown>): number | null {
  if (typeof msg.deadline === 'number' && Number.isFinite(msg.deadline)) {
    return msg.deadline
  }
  for (const key of ['swap', 'execute_swap_operations'] as const) {
    const inner = msg[key]
    if (inner && typeof inner === 'object' && !Array.isArray(inner)) {
      const d = (inner as Record<string, unknown>).deadline
      if (typeof d === 'number' && Number.isFinite(d)) {
        return d
      }
    }
  }
  return null
}

function deadlineFromSendInnerMsg(innerB64: string): number | null {
  try {
    const parsed = JSON.parse(atob(innerB64)) as Record<string, unknown>
    return deadlineFromDecodedMsg(parsed)
  } catch {
    return null
  }
}

/**
 * Extract on-chain swap/router deadline (unix seconds) from execute entries when present.
 * Falls back to `now + defaultRecoverySeconds` for txs without an explicit deadline (GitLab #359).
 */
export function resolveTerraTxRecoveryDeadlineUnix(
  entries: TerraExecuteContractEntry[],
  nowUnixSec = Math.floor(Date.now() / 1000)
): number {
  for (const entry of entries) {
    const fromTop = deadlineFromDecodedMsg(entry.msg)
    if (fromTop != null) return fromTop

    const send = entry.msg.send
    if (send && typeof send === 'object' && !Array.isArray(send)) {
      const inner = (send as Record<string, unknown>).msg
      if (typeof inner === 'string' && inner.length > 0) {
        const fromSend = deadlineFromSendInnerMsg(inner)
        if (fromSend != null) return fromSend
      }
    }
  }

  return nowUnixSec + TERRA_TX_RECOVERY_DEFAULT_SECONDS
}
