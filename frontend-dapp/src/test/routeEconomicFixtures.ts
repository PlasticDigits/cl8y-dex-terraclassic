import type { IndexerRouteSolveResponse } from '@/types'

/** Owned public synthetic diagnostics; no wallet/actor fields. */
export function attachReferenceAttribution(quote: IndexerRouteSolveResponse): IndexerRouteSolveResponse {
  const metadata = quote.execution_metadata!
  quote.economic_attribution = {
    version: 1,
    status: 'available',
    method: 'spot_bridge_v1',
    generation: metadata.generation,
    observed_at_ms: metadata.observed_at_ms,
    expires_at_ms: metadata.expires_at_ms,
    fair_amount_out: quote.spot_amount_out,
    net_amount_out: quote.estimated_amount_out,
    components: {
      wrapper_loss: '0',
      trading_cost: '0',
      market_movement: '0',
      reference_divergence: (BigInt(quote.spot_amount_out!) - BigInt(quote.estimated_amount_out!)).toString(),
    },
    dominant_cause: quote.spot_amount_out === quote.estimated_amount_out ? 'mixed' : 'reference_divergence',
  }
  return quote
}
