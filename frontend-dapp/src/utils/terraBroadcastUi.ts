import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'

/** Button copy for an in-flight Terra broadcast mutation (GitLab #305). */
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
    default:
      return pendingFallback ?? idleLabel
  }
}
