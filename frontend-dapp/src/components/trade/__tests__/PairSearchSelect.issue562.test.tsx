import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PairSearchSelect } from '@/components/trade/PairSearchSelect'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import { PAIR_SEARCH_RESULT_LIMIT } from '@/utils/pairSearchQuery'
import type { IndexerPair, PairInfo } from '@/types'

const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
const EMBER = 'terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94'
const CORAL = 'terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena'
const RUBY = 'terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc'

const ADDR_CUSTC = 'terra1pair000000000000000000000000000custc'
const ADDR_GEM = 'terra1pair00000000000000000000000000000gem'

const factoryPairs: PairInfo[] = [
  {
    contract_addr: ADDR_CUSTC,
    liquidity_token: 'terra1lp-custc',
    asset_infos: [{ token: { contract_addr: UST1 } }, { token: { contract_addr: CUSTC } }],
  },
  {
    contract_addr: ADDR_GEM,
    liquidity_token: 'terra1lp-gem',
    asset_infos: [{ token: { contract_addr: EMBER } }, { token: { contract_addr: CORAL } }],
  },
]

function idx(addr: string, s0: string, s1: string, a0: string, a1: string): IndexerPair {
  return {
    pair_address: addr,
    asset_0: { symbol: s0, contract_addr: a0, denom: null, decimals: 6 },
    asset_1: { symbol: s1, contract_addr: a1, denom: null, decimals: 6 },
    lp_token: `${addr}lp`,
    fee_bps: 30,
    is_active: true,
    volume_quote_24h: '1000000',
  }
}

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return { ...actual, getPairs: vi.fn() }
})

describe('PairSearchSelect production hide (GitLab #562 U5 / A3)', () => {
  beforeEach(() => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [
        idx(ADDR_CUSTC, 'UST1', 'cUSTC', UST1, CUSTC),
        idx(ADDR_GEM, 'EMBER', 'CORAL', EMBER, CORAL),
        idx('terra1ruby-pair', 'RUBY', 'PEARL', RUBY, EMBER),
      ],
      total: 3,
      limit: PAIR_SEARCH_RESULT_LIMIT,
      offset: 0,
    })
  })

  afterEach(() => {
    vi.unstubAllEnvs()
    vi.clearAllMocks()
  })

  it('empty browse omits gem pairs and the Test pairs divider', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <PairSearchSelect value="" onChange={vi.fn()} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    await user.click(screen.getByRole('combobox', { name: 'Trading pair' }))
    const listbox = await screen.findByRole('listbox')
    await waitFor(() => {
      expect(within(listbox).getAllByRole('option').length).toBeGreaterThan(0)
    })

    const labels = within(listbox)
      .getAllByRole('option')
      .map((el) => el.textContent ?? '')
    expect(labels.some((t) => /EMBER|CORAL|RUBY|PEARL/i.test(t))).toBe(false)
    expect(labels.some((t) => /UST1/i.test(t))).toBe(true)
    expect(within(listbox).queryByText(/Test pairs/i)).not.toBeInTheDocument()
  })

  it('typed search RUBY does not surface a gem pair (A2)', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <PairSearchSelect value="" onChange={vi.fn()} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    await user.click(screen.getByRole('combobox', { name: 'Trading pair' }))
    await user.type(screen.getByRole('combobox', { name: 'Trading pair' }), 'RUBY')
    await waitFor(() => {
      expect(indexerClient.getPairs).toHaveBeenCalled()
    })
    const listbox = screen.getByRole('listbox')
    const labels = within(listbox)
      .queryAllByRole('option')
      .map((el) => el.textContent ?? '')
    expect(labels.some((t) => /RUBY/i.test(t))).toBe(false)
  })
})
