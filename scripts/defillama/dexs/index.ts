/**
 * dimension-adapters copy: `dexs/cl8y-dex/index.ts`
 *
 * Submit upstream to https://github.com/DefiLlama/dimension-adapters
 * Test there: `pnpm test dexs cl8y-dex`
 *
 * Version 1 — GET /api/v1/defillama/daily is a UTC calendar-day rollup and
 * cannot be split hourly. Host is pinned (A18).
 *
 * In-repo unit tests use `../dimensions/mapDaily.js`, not this TypeScript file.
 */

import { FetchOptions, SimpleAdapter } from '../../adapters/types'
import { CHAIN } from '../../helpers/chains'
import {
  ADAPTER_START,
  dailyUrl,
  mapVolume,
  METHODOLOGY,
  INDEXER_DAILY_URL,
} from '../dimensions/mapDaily'

const fetch = async (options: FetchOptions) => {
  const url = dailyUrl(options.startOfDay, INDEXER_DAILY_URL)
  const res = await options.http.get(url)
  const mapped = mapVolume(res)
  if (mapped.dailyVolume == null) {
    throw new Error(`cl8y-dex dailyVolume unpriced or missing for ${options.startOfDay}`)
  }
  return { dailyVolume: mapped.dailyVolume }
}

const adapter: SimpleAdapter = {
  version: 1,
  fetch,
  chains: [CHAIN.TERRA],
  start: ADAPTER_START,
  methodology: {
    Volume: METHODOLOGY.Volume,
  },
}

export default adapter
