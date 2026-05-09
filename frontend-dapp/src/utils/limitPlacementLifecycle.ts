import type { IndexerLimitPlacement, PairInfo } from '@/types'
import { assetInfoLabel, tokenAssetInfo } from '@/types'
import { formatTokenAmount, getDecimals } from '@/utils/formatAmount'

/** Normalized lifecycle from indexer (`GET .../limit-placements`). Legacy rows without the field count as active. */
export function normalizedLimitPlacementLifecycle(
  row: IndexerLimitPlacement
): 'active' | 'parked_expired' | 'refunded' {
  const s = row.lifecycle_status?.trim().toLowerCase()
  if (s === 'parked_expired' || s === 'refunded') return s
  return 'active'
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
  }
  const sortDesc = (a: IndexerLimitPlacement, b: IndexerLimitPlacement) => b.order_id - a.order_id
  active.sort(sortDesc)
  parkedExpired.sort(sortDesc)
  return { active, parkedExpired }
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
