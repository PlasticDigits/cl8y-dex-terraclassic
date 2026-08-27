/**
 * dimension-adapters copy: `dexs/cl8y-dex/index.ts`
 *
 * Submit upstream to https://github.com/DefiLlama/dimension-adapters
 * Test there: `pnpm test dexs cl8y-dex`
 *
 * Version 1 — GET /api/v1/defillama/daily is a UTC calendar-day rollup and
 * cannot be split hourly. Host is pinned (A18). `"0"` is valid; JSON null throws.
 *
 * In-repo unit tests use `../dimensions/mapDaily.js`, not this TypeScript file.
 */

import { FetchOptions, SimpleAdapter } from '../../adapters/types'
import { CHAIN } from '../../helpers/chains'
import {
  ADAPTER_START_ISO,
  dailyUrl,
  mapVolume,
  METHODOLOGY,
  INDEXER_DAILY_URL,
  requirePricedUsd,
} from '../dimensions/mapDaily'

const fetch = async (options: FetchOptions) => {
  const url = dailyUrl(options.startOfDay, INDEXER_DAILY_URL)
  const res = await options.http.get(url)
  const mapped = mapVolume(res)
  const dailyVolume = requirePricedUsd(
    mapped.dailyVolume,
    'dailyVolume',
    options.startOfDay,
  )
  return { dailyVolume }
}

const adapter: SimpleAdapter = {
  version: 1,
  fetch,
  chains: [CHAIN.TERRA],
  start: ADAPTER_START_ISO,
  methodology: {
    Volume: METHODOLOGY.Volume,
  },
}

export default adapter
