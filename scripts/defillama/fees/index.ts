/**
 * dimension-adapters copy: `fees/cl8y-dex/index.ts`
 *
 * Submit upstream to https://github.com/DefiLlama/dimension-adapters
 * Test there: `pnpm test fees cl8y-dex`
 *
 * Version 1 — GET /api/v1/defillama/daily is a UTC calendar-day rollup.
 * SSR is 0. Breakdown labels map to Llama METRIC enum in the upstream file.
 *
 * In-repo unit tests use `../dimensions/mapDaily.js`, not this TypeScript file.
 */

import { FetchOptions, SimpleAdapter } from '../../adapters/types'
import { CHAIN } from '../../helpers/chains'
import {
  ADAPTER_START,
  BREAKDOWN_METHODOLOGY,
  dailyUrl,
  mapFees,
  METHODOLOGY,
  INDEXER_DAILY_URL,
} from '../dimensions/mapDaily'

const fetch = async (options: FetchOptions) => {
  const url = dailyUrl(options.startOfDay, INDEXER_DAILY_URL)
  const res = await options.http.get(url)
  const mapped = mapFees(res)
  if (mapped.dailyFees == null) {
    throw new Error(`cl8y-dex dailyFees unpriced or missing for ${options.startOfDay}`)
  }
  const dailyFees = options.createBalances()
  const dailyRevenue = options.createBalances()
  dailyFees.addUSDValue(mapped.dailyFees)
  dailyRevenue.addUSDValue(mapped.dailyRevenue ?? mapped.dailyFees)
  for (const [label, value] of Object.entries(mapped.breakdown)) {
    if (value && value > 0) {
      dailyFees.addUSDValue(value, label)
    }
  }
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
  start: ADAPTER_START,
  methodology: {
    Fees: METHODOLOGY.Fees,
    Revenue: METHODOLOGY.Revenue,
    ProtocolRevenue: METHODOLOGY.ProtocolRevenue,
    SupplySideRevenue: METHODOLOGY.SupplySideRevenue,
  },
  breakdownMethodology: {
    Fees: BREAKDOWN_METHODOLOGY,
  },
}

export default adapter
