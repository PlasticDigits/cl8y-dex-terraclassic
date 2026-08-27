/**
 * dimension-adapters copy: `fees/cl8y-dex/index.ts`
 *
 * Submit upstream to https://github.com/DefiLlama/dimension-adapters
 * Test there: `pnpm test fees cl8y-dex`
 *
 * Version 1 — GET /api/v1/defillama/daily is a UTC calendar-day rollup.
 * SSR is 0. Adds Llama METRIC groups + residual only (never total then labels).
 * `"0"` is valid. JSON `null` (all-unpriced) throws. Do not map null → $0.
 *
 * In-repo unit tests use `../dimensions/mapDaily.js`, not this TypeScript file.
 */

import { FetchOptions, SimpleAdapter } from '../../adapters/types'
import { CHAIN } from '../../helpers/chains'
import { METRIC } from '../../helpers/metrics'
import {
  ADAPTER_START_ISO,
  dailyUrl,
  feeMetricGroups,
  feeResidual,
  LLAMA_FEE_METRICS,
  mapFees,
  METHODOLOGY,
  INDEXER_DAILY_URL,
  requirePricedUsd,
} from '../dimensions/mapDaily'

const fetch = async (options: FetchOptions) => {
  const url = dailyUrl(options.startOfDay, INDEXER_DAILY_URL)
  const res = await options.http.get(url)
  const mapped = mapFees(res)
  const dailyFeesUsd = requirePricedUsd(
    mapped.dailyFees,
    'dailyFees',
    options.startOfDay,
  )
  const groups = feeMetricGroups(mapped.breakdown)
  const residual = feeResidual(dailyFeesUsd, mapped.breakdown) ?? 0

  const dailyFees = options.createBalances()
  const dailyRevenue = options.createBalances()
  if (groups.swapFees) dailyFees.addUSDValue(groups.swapFees, METRIC.SWAP_FEES)
  if (groups.wrapFees) dailyFees.addUSDValue(groups.wrapFees, METRIC.DEPOSIT_WITHDRAW_FEES)
  if (groups.mintRedeemFees) {
    dailyFees.addUSDValue(groups.mintRedeemFees, METRIC.MINT_REDEEM_FEES)
  }
  if (residual > 0) dailyFees.addUSDValue(residual)
  dailyRevenue.addUSDValue(mapped.dailyRevenue ?? dailyFeesUsd)

  return {
    dailyFees,
    dailyRevenue,
    dailyProtocolRevenue: dailyRevenue,
    dailySupplySideRevenue: 0,
  }
}

const adapter: SimpleAdapter = {
  version: 1,
  fetch,
  chains: [CHAIN.TERRA],
  start: ADAPTER_START_ISO,
  methodology: {
    Fees: METHODOLOGY.Fees,
    Revenue: METHODOLOGY.Revenue,
    ProtocolRevenue: METHODOLOGY.ProtocolRevenue,
    SupplySideRevenue: METHODOLOGY.SupplySideRevenue,
  },
  breakdownMethodology: {
    Fees: {
      [METRIC.SWAP_FEES]: LLAMA_FEE_METRICS.SWAP_FEES,
      [METRIC.DEPOSIT_WITHDRAW_FEES]: LLAMA_FEE_METRICS.DEPOSIT_WITHDRAW_FEES,
      [METRIC.MINT_REDEEM_FEES]: LLAMA_FEE_METRICS.MINT_REDEEM_FEES,
    },
  },
}

export default adapter
