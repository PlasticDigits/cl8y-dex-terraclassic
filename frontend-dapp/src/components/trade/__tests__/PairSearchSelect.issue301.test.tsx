import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PairSearchSelect } from '@/components/trade/PairSearchSelect'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import { PAIR_SEARCH_RESULT_LIMIT } from '@/utils/pairSearchQuery'
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
  asset_0: { symbol: 'AAA', contract_addr: 'terra1aaa0000000000000000000000000000001', denom: null, decimals: 6 },
  asset_1: { symbol: 'BBB', contract_addr: 'terra1bbb0000000000000000000000000000002', denom: null, decimals: 6 },
  lp_token: 'terra1lp000000000000000000000000000000001',
  fee_bps: 30,
  volume_quote_24h: '1000',
  is_active: true,
}

const indexerPairB: IndexerPair = {
  pair_address: PAIR_B,
  asset_0: { symbol: 'CCC', contract_addr: 'terra1ccc0000000000000000000000000000003', denom: null, decimals: 6 },
  asset_1: { symbol: 'DDD', contract_addr: 'terra1ddd0000000000000000000000000000004', denom: null, decimals: 6 },
  lp_token: 'terra1lp000000000000000000000000000000002',
  fee_bps: 30,
  volume_quote_24h: '500',
  is_active: true,
}

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return { ...actual, getPairs: vi.fn() }
})

describe('PairSearchSelect (GitLab #301)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [indexerPairA, indexerPairB],
      total: 2,
      limit: PAIR_SEARCH_RESULT_LIMIT,
      offset: 0,
    })
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('renders a searchable combobox instead of a static pair dropdown', () => {
    renderWithProviders(
      <PairSearchSelect value={PAIR_A} onChange={vi.fn()} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    const input = screen.getByRole('combobox', { name: 'Trading pair' })
    expect(input).toHaveAttribute('type', 'text')
    expect(input).toHaveAttribute('placeholder', 'Search pairs…')
  })

  it('loads top pairs by 24h volume when opened with no search text', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <PairSearchSelect value={PAIR_A} onChange={vi.fn()} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    await user.click(screen.getByRole('combobox', { name: 'Trading pair' }))

    await waitFor(() => {
      expect(indexerClient.getPairs).toHaveBeenCalledWith({
        q: undefined,
        sort: 'volume_24h',
        order: 'desc',
        limit: PAIR_SEARCH_RESULT_LIMIT,
      })
    })

    const listbox = await screen.findByRole('listbox')
    await waitFor(() => {
      const options = within(listbox).getAllByRole('option')
      expect(options).toHaveLength(2)
      expect(options[0]).toHaveTextContent(/AAA/i)
      expect(options[1]).toHaveTextContent(/CCC/i)
    })
  })

  it('calls onChange when a different pair is selected', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    renderWithProviders(
      <PairSearchSelect value={PAIR_A} onChange={onChange} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    await user.click(screen.getByRole('combobox', { name: 'Trading pair' }))
    const listbox = await screen.findByRole('listbox')
    await user.click(within(listbox).getByRole('option', { name: /CCC/i }))

    expect(onChange).toHaveBeenCalledWith(PAIR_B)
  })
})
