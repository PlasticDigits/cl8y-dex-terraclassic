/** Local observation fixtures shared by migrated market page regressions (#1452).
 * `catalog` reads synthetic test data only; no factory/network discovery is installed.
 */
import { vi } from 'vitest'
import * as client from '@/services/indexer/client'
import type { PairInfo, IndexerPair, IndexerToken } from '@/types'
import { localQuote } from './localRouteFixtures'

export function localMarketMocks(
  catalog: () => Promise<{ pairs: PairInfo[] }>,
  options?: {
    decimals?: (token: string) => number | null
    preservePairMock?: boolean
    output?: (amount: string, from: string, to: string) => string
  }
) {
  const asset = (a: PairInfo['asset_infos'][number]): IndexerPair['asset_0'] => {
    const id = 'token' in a ? a.token.contract_addr : a.native_token.denom
    return {
      contract_addr: 'token' in a ? id : null,
      denom: 'native_token' in a ? id : null,
      symbol: 'fixture',
      // Null intentionally represents unavailable metadata in negative fixtures.
      decimals: (options?.decimals ? options.decimals(id) : 6) as number,
    }
  }
  const pairs = async (): Promise<IndexerPair[]> =>
    (await catalog()).pairs.map((p) => ({
      pair_address: p.contract_addr,
      lp_token: p.liquidity_token,
      fee_bps: 30,
      is_active: true,
      asset_0: asset(p.asset_infos[0]),
      asset_1: asset(p.asset_infos[1]),
    }))
  vi.spyOn(client, 'getAllPairsPaged').mockImplementation(pairs)
  if (!options?.preservePairMock)
    vi.spyOn(client, 'getPair').mockImplementation(async (address) => {
      const pair = (await pairs()).find((p) => p.pair_address === address)
      if (!pair) throw new Error('Unknown pair')
      return pair
    })
  vi.spyOn(client, 'getTokens').mockImplementation(async () =>
    (await pairs())
      .flatMap((p) => [p.asset_0, p.asset_1])
      .map(
        (a, i): IndexerToken => ({
          id: i,
          contract_address: a.contract_addr,
          denom: a.denom,
          is_cw20: !!a.contract_addr,
          decimals: a.decimals,
          symbol: a.symbol,
          name: a.symbol,
          logo_url: null,
          coingecko_id: null,
          cmc_id: null,
        })
      )
  )
  vi.spyOn(client, 'getRouteSolve').mockImplementation(async (from, to, amount, constraints) => {
    if (!amount) throw new Error('Missing exact input')
    const result = localQuote(from, to, amount, constraints, options?.output?.(amount, from, to) ?? amount)
    const pair = (await pairs()).find(
      (p) =>
        [p.asset_0, p.asset_1].some((a) => (a.contract_addr ?? a.denom) === from) &&
        [p.asset_0, p.asset_1].some((a) => (a.contract_addr ?? a.denom) === to)
    )
    if (!pair) throw new Error('Quote unavailable')
    result.hops[0].pair = pair.pair_address
    result.execution_metadata!.hops[0].pair = pair.pair_address
    return result
  })
}
