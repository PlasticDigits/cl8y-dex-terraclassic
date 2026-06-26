import { describe, it, expect } from 'vitest'
import {
  escrowTokenAddressForLimitSide,
  formatRemainingEscrowHuman,
  isParkedDustPlacement,
  normalizedLimitPlacementLifecycle,
  parkedClaimButtonLabel,
  partitionLimitPlacementsByLifecycle,
  partitionParkedPlacementsByKind,
} from '@/utils/limitPlacementLifecycle'
import type { IndexerLimitPlacement, PairInfo } from '@/types'

const T0 = 'terra1token0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const T1 = 'terra1token1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'

const mockPair: PairInfo = {
  contract_addr: 'terra1pair',
  liquidity_token: 'terra1lp',
  asset_infos: [{ token: { contract_addr: T0 } }, { token: { contract_addr: T1 } }],
}

function row(
  partial: Partial<IndexerLimitPlacement> & Pick<IndexerLimitPlacement, 'id' | 'order_id'>
): IndexerLimitPlacement {
  return {
    pair_address: 'terra1pair',
    block_height: 1,
    block_timestamp: '2026-01-01T00:00:00Z',
    tx_hash: 'ABCD',
    owner: 'terra1maker',
    side: 'bid',
    price: '1',
    ...partial,
  }
}

describe('normalizedLimitPlacementLifecycle', () => {
  it('defaults missing lifecycle to active', () => {
    expect(normalizedLimitPlacementLifecycle(row({ id: 1, order_id: 1 }))).toBe('active')
  })

  it('parses parked_expired', () => {
    expect(normalizedLimitPlacementLifecycle(row({ id: 1, order_id: 1, lifecycle_status: 'parked_expired' }))).toBe(
      'parked_expired'
    )
  })
})

describe('partitionLimitPlacementsByLifecycle', () => {
  it('splits active vs parked and sorts by order_id desc', () => {
    const { active, parkedExpired } = partitionLimitPlacementsByLifecycle([
      row({ id: 1, order_id: 1, lifecycle_status: 'active' }),
      row({ id: 2, order_id: 10, lifecycle_status: 'parked_expired' }),
      row({ id: 3, order_id: 5, lifecycle_status: 'active' }),
    ])
    expect(active.map((r) => r.order_id)).toEqual([5, 1])
    expect(parkedExpired.map((r) => r.order_id)).toEqual([10])
  })

  it('drops refunded rows from both buckets (default UI feed excludes them anyway)', () => {
    const { active, parkedExpired } = partitionLimitPlacementsByLifecycle([
      row({ id: 1, order_id: 1, lifecycle_status: 'refunded' }),
    ])
    expect(active).toEqual([])
    expect(parkedExpired).toEqual([])
  })
})

describe('escrowTokenAddressForLimitSide', () => {
  it('maps bid to token1 and ask to token0', () => {
    expect(escrowTokenAddressForLimitSide(mockPair, 'bid')).toBe(T1)
    expect(escrowTokenAddressForLimitSide(mockPair, 'ask')).toBe(T0)
  })
})

describe('formatRemainingEscrowHuman', () => {
  it('formats raw remaining against escrow token decimals', () => {
    const r = row({
      id: 1,
      order_id: 3,
      lifecycle_status: 'parked_expired',
      side: 'bid',
      remaining_escrow: '1000000',
    })
    expect(formatRemainingEscrowHuman(r, mockPair)).not.toBe('—')
  })
})

describe('parked dust vs expiry (#419)', () => {
  it('detects dust when remaining_escrow is below threshold', () => {
    expect(isParkedDustPlacement(row({ id: 1, order_id: 1, remaining_escrow: '5' }))).toBe(true)
    expect(isParkedDustPlacement(row({ id: 2, order_id: 2, remaining_escrow: '10' }))).toBe(false)
    expect(parkedClaimButtonLabel(row({ id: 1, order_id: 1, remaining_escrow: '3' }))).toBe('Claim dust')
    expect(parkedClaimButtonLabel(row({ id: 2, order_id: 2, remaining_escrow: '1000' }))).toBe('Claim refund')
  })

  it('partitions parked rows into expired vs dust buckets', () => {
    const { expired, dust } = partitionParkedPlacementsByKind([
      row({ id: 1, order_id: 1, lifecycle_status: 'parked_expired', remaining_escrow: '2' }),
      row({ id: 2, order_id: 2, lifecycle_status: 'parked_expired', remaining_escrow: '5000' }),
    ])
    expect(dust.map((r) => r.order_id)).toEqual([1])
    expect(expired.map((r) => r.order_id)).toEqual([2])
  })
})
