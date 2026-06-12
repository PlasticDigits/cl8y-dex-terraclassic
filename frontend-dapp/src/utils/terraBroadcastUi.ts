import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE } from '@/utils/terraTxTimeout'

/** Button copy for an in-flight Terra broadcast mutation (GitLab #305, #359). */
export function terraBroadcastPendingButtonLabel(
  phase: TerraBroadcastPhase | null,
  isPending: boolean,
  idleLabel: string,
  pendingFallback?: string
): string {
  if (!isPending) return idleLabel
  switch (phase) {
    case 'signing':
      return 'Signing…'
    case 'broadcasting':
      return 'Broadcasting…'
    case 'confirming':
      return 'Confirming…'
    case 'recovering':
      return 'Checking broadcast…'
    default:
      return pendingFallback ?? idleLabel
  }
}

/** In-flight status when post-sign broadcast outcome is unknown (GitLab #359). */
export function terraBroadcastPendingStatusMessage(phase: TerraBroadcastPhase | null): string | null {
  if (phase === 'recovering') {
    return TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE
  }
  return null
}
