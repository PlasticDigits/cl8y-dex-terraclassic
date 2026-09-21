import type { SwapOperation } from '@/services/terraclassic/router'
import type { HybridSwapParams } from '@/types'

/**
 * Declared Pattern C hybrid is a partition of that hop’s offer (#1280).
 * Retail GET keeps hybrid on hop 0 only; wrap/native BFS never copies hybrid (#1264).
 */

export class HybridHopOfferPartitionError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'HybridHopOfferPartitionError'
  }
}

export const HYBRID_HOP0_PARTITION_USER_MESSAGE =
  'This route’s book/pool split does not match the amount you are sending. Refresh the quote and try again.'

export function parseRawUint(s: string): bigint | null {
  if (!/^\d+$/.test(s.trim())) return null
  try {
    return BigInt(s.trim())
  } catch {
    return null
  }
}

export function hybridDeclaredSum(h: HybridSwapParams): bigint | null {
  const pool = parseRawUint(h.pool_input)
  const book = parseRawUint(h.book_input)
  if (pool === null || book === null) return null
  return pool + book
}

export function hybridPartitionsHopOffer(h: HybridSwapParams, hopOffer: string): boolean {
  const sum = hybridDeclaredSum(h)
  const offer = parseRawUint(hopOffer)
  if (sum === null || offer === null) return false
  return sum === offer
}

/** Policy A: drop declared hybrid on hops 1+ so execute cannot freeze quote-time interiors. */
export function stripInteriorDeclaredHybrid(ops: SwapOperation[]): SwapOperation[] {
  return ops.map((op, i) => {
    if (i === 0 || !op.terra_swap.hybrid) return op
    return {
      terra_swap: {
        offer_asset_info: op.terra_swap.offer_asset_info,
        ask_asset_info: op.terra_swap.ask_asset_info,
        min_return: op.terra_swap.min_return,
      },
    }
  })
}

/** Fail closed before sign: hop 0 declared hybrid must partition the CW20 send amount. */
export function assertHop0DeclaredHybridPartitionsOffer(ops: SwapOperation[], hop0Offer: string): void {
  const hop0 = ops[0]
  if (!hop0?.terra_swap.hybrid) return
  if (!hybridPartitionsHopOffer(hop0.terra_swap.hybrid, hop0Offer)) {
    throw new HybridHopOfferPartitionError(HYBRID_HOP0_PARTITION_USER_MESSAGE)
  }
}
