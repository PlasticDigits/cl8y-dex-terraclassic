import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(),
}))

vi.mock('@/services/terraclassic/transactions', () => ({
  executeTerraContract: vi.fn(),
}))

vi.mock('@/utils/constants', () => ({
  FACTORY_CONTRACT_ADDRESS: 'terra1factory',
  FEE_DISCOUNT_CONTRACT_ADDRESS: 'terra1feediscount',
  TERRA_LCD_URL: 'http://localhost:1317',
}))

import { queryContract } from '@/services/terraclassic/queries'
import { executeTerraContract } from '@/services/terraclassic/transactions'
import { getAllPairs, getAllPairsPaginated, getPair, getWhitelistedCodeIds, createPair } from '../factory'
import type { AssetInfo, PairInfo } from '@/types'

const mockedQuery = vi.mocked(queryContract)
const mockedExecute = vi.mocked(executeTerraContract)

const FACTORY = 'terra1factory'
const WALLET = 'terra1wallet'
const TOKEN_A = 'terra1tokena'
const TOKEN_B = 'terra1tokenb'

function makePairInfo(addr: string, a: string, b: string): PairInfo {
  return {
    asset_infos: [{ token: { contract_addr: a } }, { token: { contract_addr: b } }],
    contract_addr: addr,
    liquidity_token: `${addr}_lp`,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('getAllPairs', () => {
  it('queries pairs without pagination params', async () => {
    const expected = { pairs: [makePairInfo('pair1', TOKEN_A, TOKEN_B)] }
    mockedQuery.mockResolvedValueOnce(expected)

    const result = await getAllPairs()

    expect(mockedQuery).toHaveBeenCalledWith(FACTORY, {
      pairs: { start_after: undefined, limit: undefined },
    })
    expect(result).toEqual(expected)
  })

  it('queries pairs with start_after and limit', async () => {
    const startAfter: [AssetInfo, AssetInfo] = [
      { token: { contract_addr: TOKEN_A } },
      { token: { contract_addr: TOKEN_B } },
    ]
    mockedQuery.mockResolvedValueOnce({ pairs: [] })

    await getAllPairs(startAfter, 10)

    expect(mockedQuery).toHaveBeenCalledWith(FACTORY, {
      pairs: { start_after: startAfter, limit: 10 },
    })
  })
})

describe('getAllPairsPaginated', () => {
  it('returns all pairs from a single page when fewer than page size', async () => {
    const pairs = [makePairInfo('pair1', TOKEN_A, TOKEN_B), makePairInfo('pair2', TOKEN_A, 'terra1c')]
    mockedQuery.mockResolvedValueOnce({ pairs })

    const result = await getAllPairsPaginated()

    expect(result.pairs).toEqual(pairs)
    expect(mockedQuery).toHaveBeenCalledTimes(1)
  })

  it('paginates through multiple pages', async () => {
    const page1 = Array.from({ length: 50 }, (_, i) => makePairInfo(`pair${i}`, TOKEN_A, `terra1t${i}`))
    const page2 = [makePairInfo('pair50', TOKEN_A, 'terra1t50')]

    mockedQuery.mockResolvedValueOnce({ pairs: page1 }).mockResolvedValueOnce({ pairs: page2 })

    const result = await getAllPairsPaginated()

    expect(result.pairs).toHaveLength(51)
    expect(mockedQuery).toHaveBeenCalledTimes(2)

    const secondCallArgs = mockedQuery.mock.calls[1]
    const queryMsg = secondCallArgs[1] as { pairs: { start_after: [AssetInfo, AssetInfo] } }
    expect(queryMsg.pairs.start_after).toEqual(page1[49].asset_infos)
  })

  it('returns empty pairs when first page is empty', async () => {
    mockedQuery.mockResolvedValueOnce({ pairs: [] })

    const result = await getAllPairsPaginated()

    expect(result.pairs).toEqual([])
    expect(mockedQuery).toHaveBeenCalledTimes(1)
  })

  it('respects the maxPairs limit', async () => {
    const page1 = Array.from({ length: 50 }, (_, i) => makePairInfo(`pair${i}`, TOKEN_A, `terra1t${i}`))

    mockedQuery.mockResolvedValueOnce({ pairs: page1 })

    const result = await getAllPairsPaginated(30)

    expect(result.pairs).toHaveLength(50)
    expect(mockedQuery).toHaveBeenCalledTimes(1)
  })
})

describe('getPair', () => {
  it('queries a specific pair by asset infos', async () => {
    const assetInfos: [AssetInfo, AssetInfo] = [
      { token: { contract_addr: TOKEN_A } },
      { token: { contract_addr: TOKEN_B } },
    ]
    const pairInfo = makePairInfo('pair1', TOKEN_A, TOKEN_B)
    mockedQuery.mockResolvedValueOnce({ pair: pairInfo })

    const result = await getPair(assetInfos)

    expect(mockedQuery).toHaveBeenCalledWith(FACTORY, {
      pair: { asset_infos: assetInfos },
    })
    expect(result).toEqual(pairInfo)
  })
})

describe('getWhitelistedCodeIds', () => {
  it('queries whitelisted code IDs without pagination', async () => {
    const expected = { code_ids: [1, 2, 3], next: null }
    mockedQuery.mockResolvedValueOnce(expected)

    const result = await getWhitelistedCodeIds()

    expect(mockedQuery).toHaveBeenCalledWith(FACTORY, {
      get_whitelisted_code_ids: { start_after: undefined, limit: undefined },
    })
    expect(result).toEqual(expected)
  })

  it('queries whitelisted code IDs with pagination', async () => {
    const expected = { code_ids: [4, 5], next: 6 }
    mockedQuery.mockResolvedValueOnce(expected)

    const result = await getWhitelistedCodeIds(3, 2)

    expect(mockedQuery).toHaveBeenCalledWith(FACTORY, {
      get_whitelisted_code_ids: { start_after: 3, limit: 2 },
    })
    expect(result).toEqual(expected)
  })
})

describe('createPair', () => {
  it('executes create_pair with token asset infos', async () => {
    mockedExecute.mockResolvedValueOnce('txhash_create')

    const result = await createPair(WALLET, TOKEN_A, TOKEN_B)

    expect(result).toBe('txhash_create')
    expect(mockedExecute).toHaveBeenCalledWith(WALLET, FACTORY, {
      create_pair: {
        asset_infos: [{ token: { contract_addr: TOKEN_A } }, { token: { contract_addr: TOKEN_B } }],
      },
    })
  })
})
