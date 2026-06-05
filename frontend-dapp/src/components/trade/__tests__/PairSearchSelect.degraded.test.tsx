import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PairSearchSelect } from '@/components/trade/PairSearchSelect'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import type { PairInfo } from '@/types'

const EMBER_ADDR = 'terra1ember00000000000000000000000000000001'
const CORAL_ADDR = 'terra1coral00000000000000000000000000000002'
const PAIR_ADDR = 'terra1pair0000000000000000000000000000000001'

const factoryPairs: PairInfo[] = [
  {
    contract_addr: PAIR_ADDR,
    liquidity_token: 'terra1lp000000000000000000000000000000001',
    asset_infos: [{ token: { contract_addr: EMBER_ADDR } }, { token: { contract_addr: CORAL_ADDR } }],
  },
]

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return { ...actual, getPairs: vi.fn() }
})

describe('PairSearchSelect degraded mode (GitLab #328)', () => {
  const TOKEN_CACHE_KEY = 'cl8y-dex-token-info'

  beforeEach(() => {
    localStorage.setItem(
      TOKEN_CACHE_KEY,
      JSON.stringify({
        [EMBER_ADDR.toLowerCase()]: { symbol: 'EMBER', name: 'Ember' },
        [CORAL_ADDR.toLowerCase()]: { symbol: 'CORAL', name: 'Coral' },
      })
    )
    vi.mocked(indexerClient.getPairs).mockRejectedValue(new Error('Failed to fetch'))
  })

  afterEach(() => {
    localStorage.removeItem(TOKEN_CACHE_KEY)
    vi.clearAllMocks()
  })

  it('shows factory pairs matching typed symbol when indexer is down', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    renderWithProviders(
      <PairSearchSelect value="" onChange={onChange} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    const input = screen.getByRole('combobox', { name: 'Trading pair' })
    await user.click(input)
    await user.type(input, 'EMBER')

    const listbox = await screen.findByRole('listbox')
    await waitFor(() => {
      expect(within(listbox).getByRole('option', { name: /EMBER/i })).toBeInTheDocument()
    })
    expect(screen.queryByText(/No pairs match your search/i)).not.toBeInTheDocument()
    expect(within(listbox).getByText(/Offline search/i)).toBeInTheDocument()
  })

  it('shows empty state for non-matching typed query when indexer is down', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <PairSearchSelect value="" onChange={vi.fn()} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    const input = screen.getByRole('combobox', { name: 'Trading pair' })
    await user.click(input)
    await user.type(input, 'zzzzz')

    await waitFor(() => {
      expect(screen.getByText(/No pairs match your search/i)).toBeInTheDocument()
    })
  })
})
