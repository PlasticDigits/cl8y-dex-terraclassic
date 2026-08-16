import type { IndexerLimitPlacement, PairInfo } from '@/types'
import { assetInfoLabel, tokenAssetInfo } from '@/types'
import { formatTokenAmount, getDecimals } from '@/utils/formatAmount'

/** On-chain dust flush threshold (`LIMIT_ORDER_DUST_FLUSH_THRESHOLD` in dex-common). */
export const LIMIT_ORDER_DUST_FLUSH_THRESHOLD = 10

/** Normalized lifecycle from indexer (`GET .../limit-placements`). Legacy rows without the field count as active. */
export type LimitPlacementLifecycle = 'active' | 'parked_expired' | 'refunded' | 'filled'

export function normalizedLimitPlacementLifecycle(row: IndexerLimitPlacement): LimitPlacementLifecycle {
  const s = row.lifecycle_status?.trim().toLowerCase()
  if (s === 'parked_expired' || s === 'refunded' || s === 'filled') return s
  return 'active'
}

/** Short label for portfolio / tables. */
export function placementLifecycleLabel(row: IndexerLimitPlacement): string {
  const lc = normalizedLimitPlacementLifecycle(row)
  if (lc === 'parked_expired') return 'Expired (claimable)'
  if (lc === 'refunded') return 'Refunded'
  if (lc === 'filled') return 'Filled'
  return 'Active'
}

export function partitionLimitPlacementsByLifecycle(rows: IndexerLimitPlacement[]): {
  active: IndexerLimitPlacement[]
  parkedExpired: IndexerLimitPlacement[]
} {
  const active: IndexerLimitPlacement[] = []
  const parkedExpired: IndexerLimitPlacement[] = []
  for (const r of rows) {
    const lc = normalizedLimitPlacementLifecycle(r)
    if (lc === 'parked_expired') parkedExpired.push(r)
    else if (lc === 'active') active.push(r)
    // `filled` / `refunded` are not cancelable open rows (#530).
  }
  const sortDesc = (a: IndexerLimitPlacement, b: IndexerLimitPlacement) => b.order_id - a.order_id
  active.sort(sortDesc)
  parkedExpired.sort(sortDesc)
  return { active, parkedExpired }
}

/** True when indexed `remaining_escrow` is below the on-chain dust flush threshold. */
export function isParkedDustPlacement(row: IndexerLimitPlacement): boolean {
  const raw = row.remaining_escrow?.trim()
  if (!raw || raw === '0') return false
  try {
    const n = BigInt(raw)
    return n > 0n && n < BigInt(LIMIT_ORDER_DUST_FLUSH_THRESHOLD)
  } catch {
    return false
  }
}

export function parkedClaimButtonLabel(row: IndexerLimitPlacement): 'Claim dust' | 'Claim refund' {
  return isParkedDustPlacement(row) ? 'Claim dust' : 'Claim refund'
}

export function partitionParkedPlacementsByKind(rows: IndexerLimitPlacement[]): {
  expired: IndexerLimitPlacement[]
  dust: IndexerLimitPlacement[]
} {
  const expired: IndexerLimitPlacement[] = []
  const dust: IndexerLimitPlacement[] = []
  for (const r of rows) {
    if (isParkedDustPlacement(r)) dust.push(r)
    else expired.push(r)
  }
  return { expired, dust }
}

/** Escrow token address for display: bid → token1, ask → token0 (pair ordering). */
export function escrowTokenAddressForLimitSide(pair: PairInfo | undefined, side: string | null | undefined): string {
  if (!pair?.asset_infos?.length) return ''
  const t0 = assetInfoLabel(pair.asset_infos[0])
  const t1 = assetInfoLabel(pair.asset_infos[1])
  const s = side?.toLowerCase()
  if (s === 'ask') return t0
  return t1
}

/** Human-readable `remaining_escrow` from indexer (raw integer string). */
export function formatRemainingEscrowHuman(row: IndexerLimitPlacement, pair: PairInfo | undefined): string {
  const raw = row.remaining_escrow?.trim()
  if (!raw || raw === '0') return '—'
  const escrowAddr = escrowTokenAddressForLimitSide(pair, row.side)
  const decimals = escrowAddr.startsWith('terra1') ? getDecimals(tokenAssetInfo(escrowAddr)) : 6
  return formatTokenAmount(raw, decimals)
}
