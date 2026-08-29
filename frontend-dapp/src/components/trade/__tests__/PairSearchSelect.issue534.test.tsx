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
const CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const USTR = 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv'
const EMBER = 'terra1ember00000000000000000000000000000001'
const CORAL = 'terra1coral00000000000000000000000000000002'

const ADDR_USTR = 'terra1pair000000000000000000000000000000ustr'
const ADDR_CUSTC = 'terra1pair000000000000000000000000000custc'
const ADDR_GEM = 'terra1pair00000000000000000000000000000gem'
const ADDR_CLUNC = 'terra1pair000000000000000000000000000clunc'

const factoryPairs: PairInfo[] = [
  {
    contract_addr: ADDR_USTR,
    liquidity_token: 'terra1lp-ustr',
    asset_infos: [{ token: { contract_addr: UST1 } }, { token: { contract_addr: USTR } }],
  },
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
  {
    contract_addr: ADDR_CLUNC,
    liquidity_token: 'terra1lp-clunc',
    asset_infos: [{ token: { contract_addr: CLUNC } }, { token: { contract_addr: UST1 } }],
  },
]

function idx(
  addr: string,
  s0: string,
  s1: string,
  a0: string,
  a1: string,
  volume: string,
  decimals1: number,
  volumeUsd?: string
): IndexerPair {
  return {
    pair_address: addr,
    asset_0: { symbol: s0, contract_addr: a0, denom: null, decimals: 6 },
    asset_1: { symbol: s1, contract_addr: a1, denom: null, decimals: decimals1 },
    lp_token: `${addr}lp`,
    fee_bps: 30,
    is_active: true,
    volume_quote_24h: volume,
    volume_usd_24h: volumeUsd,
  }
}

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return { ...actual, getPairs: vi.fn() }
})

describe('PairSearchSelect catalog + quote volume (GitLab #534)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [
        idx(ADDR_USTR, 'UST1', 'USTR', UST1, USTR, '19297048000000000000', 18, '19.3'),
        idx(ADDR_CUSTC, 'UST1', 'cUSTC', UST1, CUSTC, '1000000', 6, '1'),
        idx(ADDR_GEM, 'EMBER', 'CORAL', EMBER, CORAL, '9000000000', 6),
        idx(ADDR_CLUNC, 'cLUNC', 'UST1', CLUNC, UST1, '2000000', 6, '2'),
      ],
      total: 4,
      limit: PAIR_SEARCH_RESULT_LIMIT,
      offset: 0,
    })
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('lists UST1 markets together before gems and labels the test-pair group', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <PairSearchSelect value="" onChange={vi.fn()} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    await user.click(screen.getByRole('combobox', { name: 'Trading pair' }))
    const listbox = await screen.findByRole('listbox')
    await waitFor(() => {
      expect(within(listbox).getAllByRole('option')).toHaveLength(4)
    })

    const labels = within(listbox)
      .getAllByRole('option')
      .map((el) => el.textContent ?? '')
    const cluncIdx = labels.findIndex((t) => /cLUNC/i.test(t) && /UST1/i.test(t))
    const custcIdx = labels.findIndex((t) => /cUSTC/i.test(t))
    const ustrIdx = labels.findIndex((t) => /USTR/i.test(t))
    const gemIdx = labels.findIndex((t) => /EMBER/i.test(t))
    expect(cluncIdx).toBeGreaterThanOrEqual(0)
    expect(custcIdx).toBeGreaterThanOrEqual(0)
    expect(ustrIdx).toBeGreaterThanOrEqual(0)
    expect(gemIdx).toBeGreaterThanOrEqual(0)
    expect(Math.max(cluncIdx, custcIdx, ustrIdx)).toBeLessThan(gemIdx)
    expect(within(listbox).getByText(/Test pairs/i)).toBeInTheDocument()
  })

  it('formats UST1/USTR 24h volume as compact USD, not 18-dec quote T', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <PairSearchSelect value="" onChange={vi.fn()} factoryPairs={factoryPairs} aria-label="Trading pair" />
    )

    await user.click(screen.getByRole('combobox', { name: 'Trading pair' }))
    const listbox = await screen.findByRole('listbox')
    const ustrRow = await waitFor(() => {
      const hit = within(listbox)
        .getAllByRole('option')
        .find((el) => /USTR/i.test(el.textContent ?? ''))
      expect(hit).toBeTruthy()
      return hit!
    })
    expect(ustrRow.textContent).not.toMatch(/19,297,048T/i)
    expect(ustrRow.textContent).not.toMatch(/\d+T/)
    expect(ustrRow.textContent).toMatch(/vol\s+\$/)
  })
})
