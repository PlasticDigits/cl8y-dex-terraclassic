import { describe, it, expect, vi, beforeEach } from 'vitest'
import * as indexerClient from '@/services/indexer/client'
import * as queries from '@/services/terraclassic/queries'
import { fetchPortfolioLpRowsForTest } from '../usePortfolioLpBalances'

vi.mock('@/services/indexer/client', () => ({
  getPairs: vi.fn(),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  getTokenBalance: vi.fn(),
}))

const VALID_PAIR = 'terra146ypndztcmmrmyxef7e20cul82gh43vjnw4uacwdvg5sp9kva7sqc9mjav'
const VALID_LP = 'terra1wl59k23zngj34l7d42y9yltask7rjlnxgccawc7ltrknp6n52fps4umj85'
const WALLET = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

describe('fetchPortfolioLpRows (GitLab #212)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [
        {
          pair_address: 'terra1paircontractabc',
          lp_token: 'terra1lptoken',
          asset_0: { symbol: 'BAD', contract_addr: 'terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' },
          asset_1: { symbol: 'PAIR', contract_addr: 'terra1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' },
          is_active: true,
          fee_bps: 30,
        },
        {
          pair_address: VALID_PAIR,
          lp_token: VALID_LP,
          asset_0: { symbol: 'EMBER', contract_addr: 'terra1cccccccccccccccccccccccccccccccccccc' },
          asset_1: { symbol: 'CORAL', contract_addr: 'terra1dddddddddddddddddddddddddddddddddddd' },
          is_active: true,
          fee_bps: 30,
        },
      ],
      total: 2,
      limit: 50,
      offset: 0,
    })
    vi.mocked(queries.getTokenBalance).mockResolvedValue('1000000')
  })

  it('skips invalid lp_token addresses and tolerates LCD errors', async () => {
    vi.mocked(queries.getTokenBalance).mockImplementation(async (_wallet, asset) => {
      const addr = 'token' in asset ? asset.token.contract_addr : ''
      if (addr === VALID_LP) return '1000000'
      throw new Error('LCD failed')
    })

    const result = await fetchPortfolioLpRowsForTest(WALLET)

    expect(result.pairsScanned).toBe(1)
    expect(result.rows).toHaveLength(1)
    expect(result.rows[0]?.pairAddress).toBe(VALID_PAIR)
    expect(queries.getTokenBalance).toHaveBeenCalledTimes(1)
  })
})
