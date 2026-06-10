import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PairSearchSelect } from '@/components/trade/PairSearchSelect'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import { PAIR_SEARCH_DEBOUNCE_MS, PAIR_SEARCH_RESULT_LIMIT } from '@/utils/pairSearchQuery'
import type { IndexerPair, PairInfo } from '@/types'

const PAIR_A = 'terra1pair0000000000000000000000000000000001'
const PAIR_B = 'terra1pair0000000000000000000000000000000002'

const factoryPairs: PairInfo[] = [
  {
    contract_addr: PAIR_A,
    liquidity_token: 'terra1lp000000000000000000000000000000001',
    asset_infos: [
      { token: { contract_addr: 'terra1aaa0000000000000000000000000000001' } },
      { token: { contract_addr: 'terra1bbb0000000000000000000000000000002' } },
    ],
  },
  {
    contract_addr: PAIR_B,
    liquidity_token: 'terra1lp000000000000000000000000000000002',
    asset_infos: [
      { token: { contract_addr: 'terra1ccc0000000000000000000000000000003' } },
      { token: { contract_addr: 'terra1ddd0000000000000000000000000000004' } },
    ],
  },
]

const indexerPairA: IndexerPair = {
  pair_address: PAIR_A,
  asset_0: { symbol: 'EMBER', contract_addr: 'terra1aaa0000000000000000000000000000001', denom: null, decimals: 6 },
  asset_1: { symbol: 'CORAL', contract_addr: 'terra1bbb0000000000000000000000000000002', denom: null, decimals: 6 },
  lp_token: 'terra1lp000000000000000000000000000000001',
  fee_bps: 30,
  volume_quote_24h: '1000',
  is_active: true,
}

const indexerPairB: IndexerPair = {
  pair_address: PAIR_B,
  asset_0: { symbol: 'JADE', contract_addr: 'terra1ccc0000000000000000000000000000003', denom: null, decimals: 6 },
  asset_1: { symbol: 'RUBY', contract_addr: 'terra1ddd0000000000000000000000000000004', denom: null, decimals: 6 },
  lp_token: 'terra1lp000000000000000000000000000000002',
  fee_bps: 30,
  volume_quote_24h: '500',
  is_active: true,
}

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return { ...actual, getPairs: vi.fn() }
})

describe('PairSearchSelect type+Enter (GitLab #350)', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.mocked(indexerClient.getPairs).mockImplementation(async (_params) => {
      const q = String(_params?.q ?? '').toLowerCase()
      if (q.includes('jade')) {
        return { items: [indexerPairB], total: 1, limit: PAIR_SEARCH_RESULT_LIMIT, offset: 0 }
      }
      return { items: [indexerPairA], total: 1, limit: PAIR_SEARCH_RESULT_LIMIT, offset: 0 }
    })
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.clearAllMocks()
  })

  it('Enter commits the first search hit when query excludes the current pair', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    const onChange = vi.fn()

    renderWithProviders(
      <PairSearchSelect value={PAIR_A} onChange={onChange} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    const input = screen.getByRole('combobox', { name: 'Trading pair' })
    await user.click(input)
    await user.type(input, 'jade')

    await vi.advanceTimersByTimeAsync(PAIR_SEARCH_DEBOUNCE_MS + 50)

    await waitFor(() => {
      expect(indexerClient.getPairs).toHaveBeenCalled()
    })

    await user.keyboard('{Enter}')

    expect(onChange).toHaveBeenCalledWith(PAIR_B)
    expect(onChange).not.toHaveBeenCalledWith(PAIR_A)
  })
})
