import { queryContract } from './queries'
import { ROUTER_CONTRACT_ADDRESS } from '@/utils/constants'
import type { SwapOperation } from './router'
import { assetInfoLabel } from '@/types'
import { classifyCommunityTaxQueryError, parseCommunityTaxSellBps } from '@/utils/taxPreviewMaxSpend'
import { assertRouteTaxPath } from './routeTaxPath'

export const ROUTE_TAX_FUNDING_UNAVAILABLE =
  'Route tax funding could not be verified. Refresh the quote or choose another route. No transaction was resent.'

/** Live router capability, independent of Create/Manage catalog pins. Never cache across migrations. */
export async function queryRouterTaxFundingVersion(signal?: AbortSignal): Promise<1 | undefined> {
  const config = await queryContract<{ funding_version?: number }>(
    ROUTER_CONTRACT_ADDRESS,
    { config: {} },
    ...(signal ? ([signal] as const) : ([] as const))
  )
  return config.funding_version === 1 ? 1 : undefined
}

/**
 * A legacy router may only execute assets whose supported tax APIs have all rejected
 * the query. Transport/malformed results are uncertainty, never proof of zero tax.
 * The upgraded router enforces its trusted token registry and transaction budgets.
 */
export async function assertRouteTaxFunding(operations: SwapOperation[], signal?: AbortSignal): Promise<1 | undefined> {
  if (!operations.length || operations.length > 4) throw new Error(ROUTE_TAX_FUNDING_UNAVAILABLE)
  let version: 1 | undefined
  try {
    version = await queryRouterTaxFundingVersion(signal)
  } catch {
    throw new Error(ROUTE_TAX_FUNDING_UNAVAILABLE)
  }
  await assertRouteTaxPath(operations, signal)
  if (version === 1) return version
  const tokens = [
    ...new Set(
      operations.flatMap((op) => [op.terra_swap.offer_asset_info, op.terra_swap.ask_asset_info].map(assetInfoLabel))
    ),
  ].filter((token) => token.startsWith('terra1'))
  // At most five assets and three queries per asset. Sequential discovery bounds LCD concurrency.
  for (const token of tokens) {
    for (const name of ['get_config', 'tax_map', 'tax_info']) {
      signal?.throwIfAborted()
      try {
        const config = await queryContract<unknown>(
          token,
          { [name]: {} },
          ...(signal ? ([signal] as const) : ([] as const))
        )
        if (name === 'get_config') {
          const parsed = parseCommunityTaxSellBps(config)
          if (
            parsed.kind === 'tax' &&
            parsed.sellBps === 0 &&
            (config as { buy_bps?: unknown; transfer_bps?: unknown }).buy_bps === 0 &&
            (config as { transfer_bps?: unknown }).transfer_bps === 0
          )
            continue
        }
        throw new Error(ROUTE_TAX_FUNDING_UNAVAILABLE)
      } catch (error) {
        if (classifyCommunityTaxQueryError(error) !== 'not_tax') {
          throw new Error(ROUTE_TAX_FUNDING_UNAVAILABLE)
        }
      }
    }
  }
  return undefined
}

/** Token preview sizes only this hop's gross funds; it cannot authorize token identity. */
export async function previewRouteTax(input: {
  token: string
  kind: 'sell' | 'buy'
  amount: string
  trader?: string
  pair: string
  signal?: AbortSignal
}): Promise<{ economic: string; net: string }> {
  try {
    const preview = await queryContract<{
      funding_version: number
      gross: string
      economic: string
      net: string
      tax: string
    }>(
      input.token,
      {
        route_tax_preview: { kind: input.kind, amount: input.amount, trader: input.trader, pair: input.pair },
      },
      ...(input.signal ? ([input.signal] as const) : ([] as const))
    )
    if (
      preview.funding_version !== 1 ||
      preview.gross !== input.amount ||
      !/^\d{1,39}$/.test(preview.economic) ||
      !/^\d{1,39}$/.test(preview.net) ||
      !/^\d{1,39}$/.test(preview.tax) ||
      BigInt(preview.economic) > BigInt(input.amount) ||
      BigInt(preview.net) > BigInt(input.amount) ||
      [preview.gross, preview.economic, preview.net, preview.tax].some((raw) => BigInt(raw) > (1n << 128n) - 1n) ||
      (input.kind === 'sell'
        ? BigInt(preview.economic) + BigInt(preview.tax) > BigInt(input.amount) || preview.net !== preview.economic
        : BigInt(preview.net) + BigInt(preview.tax) !== BigInt(input.amount) || preview.economic !== preview.gross)
    ) {
      throw new Error(ROUTE_TAX_FUNDING_UNAVAILABLE)
    }
    return preview
  } catch (error) {
    // Honest CW20 has no swap tax. Legacy tax detection/attestation is enforced by router.
    if (classifyCommunityTaxQueryError(error) === 'not_tax') return { economic: input.amount, net: input.amount }
    throw new Error(ROUTE_TAX_FUNDING_UNAVAILABLE)
  }
}
