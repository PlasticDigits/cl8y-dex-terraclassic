/**
 * dimension-adapters copy: `dexs/cl8y-dex/index.ts`
 *
 * Submit upstream to https://github.com/DefiLlama/dimension-adapters
 * Test there: `pnpm test dexs cl8y-dex`
 *
 * GitLab #631 — dailyVolume from GET /api/v1/defillama/daily (UTC day).
 * Host is pinned. No user-supplied URL (A18).
 */

import { FetchOptions } from '../../options'
import { CHAIN } from '../../helpers/chains'
import {
  ADAPTER_START,
  dailyUrl,
  mapVolume,
  METHODOLOGY,
  INDEXER_DAILY_URL,
} from '../dimensions/mapDaily'

const fetch = async (_: unknown, _1: unknown, options: FetchOptions) => {
  const url = dailyUrl(options.startOfDay, INDEXER_DAILY_URL)
  const res = await options.http.get(url)
  const mapped = mapVolume(res)
  if (mapped.dailyVolume == null) {
    throw new Error(`cl8y-dex dailyVolume unpriced or missing for ${options.startOfDay}`)
  }
  return { dailyVolume: mapped.dailyVolume }
}

export default {
  version: 2,
  adapter: {
    [CHAIN.TERRA]: {
      fetch,
      start: ADAPTER_START,
    },
  },
  methodology: {
    Volume: METHODOLOGY.Volume,
  },
}
