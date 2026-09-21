import { describe, expect, it } from 'vitest'
import type { SwapOperation } from '@/services/terraclassic/router'
import {
  HYBRID_HOP0_PARTITION_USER_MESSAGE,
  HybridHopOfferPartitionError,
  assertHop0DeclaredHybridPartitionsOffer,
  hybridPartitionsHopOffer,
  stripInteriorDeclaredHybrid,
} from './hybridHopOfferPartition'

const token = (addr: string) => ({ token: { contract_addr: addr } })

function op(hybrid?: SwapOperation['terra_swap']['hybrid']): SwapOperation {
  return {
    terra_swap: {
      offer_asset_info: token('terra1a'),
      ask_asset_info: token('terra1b'),
      ...(hybrid ? { hybrid } : {}),
    },
  }
}

const split = (pool: string, book: string) => ({
  pool_input: pool,
  book_input: book,
  max_maker_fills: 8,
  book_start_hint: null,
})

describe('hybridHopOfferPartition (#1280)', () => {
  it('accepts pool + book == offer', () => {
    expect(hybridPartitionsHopOffer(split('700', '300'), '1000')).toBe(true)
  })

  it('rejects offer-1 and non-integers', () => {
    expect(hybridPartitionsHopOffer(split('700', '299'), '1000')).toBe(false)
    expect(hybridPartitionsHopOffer(split('7.0', '3'), '10')).toBe(false)
  })

  it('strips interior hybrid and keeps hop 0', () => {
    const stripped = stripInteriorDeclaredHybrid([op(split('800', '200')), op(split('50', '50')), op()])
    expect(stripped[0].terra_swap.hybrid?.pool_input).toBe('800')
    expect(stripped[1].terra_swap.hybrid).toBeUndefined()
    expect(stripped[2].terra_swap.hybrid).toBeUndefined()
  })

  it('wrap-shaped ops stay hybrid-free (#1264)', () => {
    const wrapOps = [op(), op()]
    expect(stripInteriorDeclaredHybrid(wrapOps).every((o) => !o.terra_swap.hybrid)).toBe(true)
    expect(() => assertHop0DeclaredHybridPartitionsOffer(wrapOps, '1000')).not.toThrow()
  })

  it('throws before sign when hop 0 sum != pay raw', () => {
    expect(() => assertHop0DeclaredHybridPartitionsOffer([op(split('600', '399'))], '1000')).toThrow(
      HybridHopOfferPartitionError
    )
    expect(() => assertHop0DeclaredHybridPartitionsOffer([op(split('600', '399'))], '1000')).toThrow(
      HYBRID_HOP0_PARTITION_USER_MESSAGE
    )
  })

  it('allows hop 0 exact partition', () => {
    expect(() => assertHop0DeclaredHybridPartitionsOffer([op(split('600', '400'))], '1000')).not.toThrow()
  })
})
