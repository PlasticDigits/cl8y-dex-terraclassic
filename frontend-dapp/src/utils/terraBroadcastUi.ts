import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import {
  TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE,
  TERRA_TX_SIGNING_HINT_DELAY_MS,
  TERRA_TX_SIGNING_KEPLR_DELAYED_HINT,
  TERRA_TX_SIGNING_LEDGER_HINT,
} from '@/utils/terraTxTimeout'

export type TerraBroadcastStatusMessageOptions = {
  isNanoLedger?: boolean
  signingElapsedMs?: number
}

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

/** In-flight status: #359 recovery, or Keplr/Ledger signing hints (GitLab #567). */
export function terraBroadcastPendingStatusMessage(
  phase: TerraBroadcastPhase | null,
  options?: TerraBroadcastStatusMessageOptions
): string | null {
  if (phase === 'recovering') {
    return TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE
  }
  if (phase === 'signing') {
    if (options?.isNanoLedger) {
      return TERRA_TX_SIGNING_LEDGER_HINT
    }
    if ((options?.signingElapsedMs ?? 0) >= TERRA_TX_SIGNING_HINT_DELAY_MS) {
      return TERRA_TX_SIGNING_KEPLR_DELAYED_HINT
    }
  }
  return null
}
