import type { PairOrderStatusKind } from '@/types'
import { normalizedLimitPlacementLifecycle, type LimitPlacementLifecycle } from '@/utils/limitPlacementLifecycle'

/**
 * Retail open-row classification for My open limits Cancel (GitLab #530).
 *
 * LCD `OrderStatus` is preferred when a successful decode exists. Transport / query
 * failure must stay `undefined` — never map it to `unknown` (**L21** / #505).
 */
export type OpenLimitReconcileKind = 'cancelable' | 'claim' | 'filled' | 'already_cancelled' | 'gone'

export interface OpenLimitReconcileInput {
  /** Successful LCD `OrderStatus`. `undefined` = not loaded or query failed (not Unknown). */
  lcdStatus: PairOrderStatusKind | undefined
  indexerLifecycle: LimitPlacementLifecycle
  hasIndexedCancellation: boolean
  hasIndexedFill: boolean
  /** Local optimistic cancel after a successful broadcast (indexer lag — I9). */
  locallyCancelled?: boolean
}

export function classifyOpenLimitRow(input: OpenLimitReconcileInput): OpenLimitReconcileKind {
  if (input.locallyCancelled || input.hasIndexedCancellation) return 'already_cancelled'
  if (input.indexerLifecycle === 'parked_expired') return 'claim'
  if (input.indexerLifecycle === 'filled') return 'filled'
  if (input.indexerLifecycle === 'refunded') return 'gone'

  if (input.lcdStatus === 'active') return 'cancelable'
  if (input.lcdStatus === 'parked_refund') return 'claim'
  if (input.lcdStatus === 'unknown') {
    // L21: Unknown ≠ fill. Classify from indexer evidence only.
    if (input.hasIndexedFill) return 'filled'
    return 'gone'
  }

  // LCD unavailable — keep Cancel on indexer-active rows (happy path + #135 humanize).
  return 'cancelable'
}

/** Marker for rows that are still cancelable on-chain. Gone/filled/cancelled must not keep a green ●. */
export function openLimitRowMarker(kind: OpenLimitReconcileKind): '●' | '◆' | '○' {
  if (kind === 'claim') return '◆'
  if (kind === 'cancelable') return '●'
  return '○'
}

export function openLimitRowStatusCopy(kind: OpenLimitReconcileKind): string | null {
  switch (kind) {
    case 'filled':
      return 'Filled'
    case 'already_cancelled':
      return 'Already cancelled'
    case 'gone':
      return 'No longer on the book'
    default:
      return null
  }
}

/**
 * Cancel / claim CTA label. `null` means hide Cancel (Claim is rendered separately).
 * Disabled reasons are explicit — never a mute "Cancel" (#530 AC5).
 */
export function openLimitCancelButtonLabel(args: {
  kind: OpenLimitReconcileKind
  isWalletConnected: boolean
  isPairPaused: boolean
  tradingRestricted: boolean
  pending: boolean
}): string | null {
  const { kind, isWalletConnected, isPairPaused, tradingRestricted, pending } = args
  if (kind === 'claim') return null
  if (!isWalletConnected) return 'Connect to cancel'
  if (kind === 'filled') return 'Filled'
  if (kind === 'already_cancelled') return 'Already cancelled'
  if (kind === 'gone') return 'No longer on the book'
  if (isPairPaused) return 'Unavailable (pair paused)'
  if (tradingRestricted) return 'Trading restricted'
  if (pending) return 'Cancelling…'
  return 'Cancel'
}

export function openLimitCancelEnabled(args: {
  kind: OpenLimitReconcileKind
  isWalletConnected: boolean
  isPairPaused: boolean
  tradingRestricted: boolean
  pending: boolean
  hasCancelMutation: boolean
}): boolean {
  if (!args.hasCancelMutation || !args.isWalletConnected) return false
  if (args.kind !== 'cancelable') return false
  if (args.isPairPaused || args.tradingRestricted || args.pending) return false
  return true
}

export function reconcilePlacementRowKind(
  row: { lifecycle_status?: string | null },
  extras: Omit<OpenLimitReconcileInput, 'indexerLifecycle'>
): OpenLimitReconcileKind {
  return classifyOpenLimitRow({
    ...extras,
    indexerLifecycle: normalizedLimitPlacementLifecycle(row),
  })
}

export function orderIdHasIndexedFill(fills: Array<{ order_id: number }>, orderId: number): boolean {
  return fills.some((f) => f.order_id === orderId)
}
