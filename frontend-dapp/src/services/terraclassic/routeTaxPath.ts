import { assetInfoLabel } from '@/types'
import { classifyCommunityTaxQueryError } from '@/utils/taxPreviewMaxSpend'
import { queryContract } from './queries'
import type { SwapOperation } from './router'

export const REPEATED_ROUTE_TAX_ASSET =
  'A route-tax token cannot appear more than once in a route. Refresh the quote or choose another route.'

/**
 * Logical positions are the first offer followed by each ask (#1390). An
 * intermediate's adjacent buy/sell is one position, regardless of rates or guards.
 * Ordinary CW20 cycles remain eligible; router attestation still decides identity.
 */
export async function assertRouteTaxPath(operations: SwapOperation[], signal?: AbortSignal): Promise<void> {
  if (!operations.length) return
  const path = [
    assetInfoLabel(operations[0].terra_swap.offer_asset_info),
    ...operations.map((op) => assetInfoLabel(op.terra_swap.ask_asset_info)),
  ]
  const seen = new Set<string>()
  const repeated = new Set<string>()
  for (const token of path) {
    if (seen.has(token) && token.startsWith('terra1')) repeated.add(token)
    seen.add(token)
  }
  // Only repeated assets need additional discovery. Never infer capability from
  // configured tax rates, manager exemption, launch-guard state or catalog pins.
  for (const token of repeated) {
    signal?.throwIfAborted()
    try {
      await queryContract(token, { route_tax_capability: {} }, ...(signal ? ([signal] as const) : ([] as const)))
    } catch (error) {
      if (classifyCommunityTaxQueryError(error) !== 'not_tax') throw error
      // Older taxed instances must also fail closed, even with zero current rates.
      for (const name of ['get_config', 'tax_map', 'tax_info']) {
        try {
          await queryContract(token, { [name]: {} }, ...(signal ? ([signal] as const) : ([] as const)))
        } catch (legacyError) {
          if (classifyCommunityTaxQueryError(legacyError) === 'not_tax') continue
          throw legacyError
        }
        throw new Error(REPEATED_ROUTE_TAX_ASSET)
      }
      continue
    }
    // A successful capability query establishes a tax implementation; malformed
    // or unsupported versions must never turn a repeated token into an ordinary one.
    throw new Error(REPEATED_ROUTE_TAX_ASSET)
  }
}
