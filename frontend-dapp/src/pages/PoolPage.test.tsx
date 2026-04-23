import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import PoolPage from './PoolPage'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import * as indexerClient from '@/services/indexer/client'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))

vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn().mockResolvedValue({ pairs: [] }),
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getPairs: vi.fn(),
    getTokens: vi.fn().mockResolvedValue([]),
  }
})

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: 'tokenA' } }, amount: '1000000' },
      { info: { token: { contract_addr: 'tokenB' } }, amount: '1000000' },
    ],
    total_share: '1000000',
  }),
  getPairFeeConfig: vi.fn().mockResolvedValue({
    commission_rate: '0.003',
  }),
  provideLiquidity: vi.fn().mockResolvedValue('txhash123'),
  withdrawLiquidity: vi.fn().mockResolvedValue('txhash123'),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({
    fee_bps: 30,
    treasury: '',
  }),
}))

vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({ discount_bps: 0 }),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

const mockIndexerPair = (addr: string) => ({
  pair_address: addr,
  asset_0: { symbol: 'A', contract_addr: 'tokenA', denom: null, decimals: 6 },
  asset_1: { symbol: 'B', contract_addr: 'tokenB', denom: null, decimals: 6 },
  lp_token: 'lp1',
  fee_bps: 30,
  is_active: true,
})

describe('PoolPage', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [],
      total: 0,
      limit: 20,
      offset: 0,
    })
    vi.mocked(getAllPairsPaginated).mockResolvedValue({ pairs: [] })
  })

  it('renders without crashing', () => {
    renderWithProviders(<PoolPage />)
    expect(screen.getByText(/liquidity pools/i)).toBeTruthy()
  })

  it('explains indexer-sourced list and shows factory vs indexer counts when data loads', async () => {
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          asset_infos: [{ token: { contract_addr: 'tokenA' } }, { token: { contract_addr: 'tokenB' } }],
          contract_addr: 'onchainPair1',
          liquidity_token: 'lp',
        },
      ],
    })
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [mockIndexerPair('onchainPair1'), mockIndexerPair('indexerOnly')],
      total: 2,
      limit: 20,
      offset: 0,
    })
    renderWithProviders(<PoolPage />)
    await waitFor(() => expect(screen.getByText(/2 pair\(s\) \(indexer total\)/i)).toBeInTheDocument())
    expect(screen.getByText(/List source:/i)).toBeInTheDocument()
    const docsLink = screen.getByText(/Data sources \(docs\)/i).closest('a')
    expect(docsLink).toHaveAttribute(
      'href',
      'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/frontend.md#liquidity-pools-list-indexer-vs-factory'
    )
    await waitFor(() => expect(screen.getByText(/1 on-chain \(factory, router graph\)/i)).toBeInTheDocument())
    expect(
      await screen.findByText(/indexer reports 2 pair\(s\) while the factory currently lists 1/i)
    ).toBeInTheDocument()
  })

  it('shows In router (factory) vs Indexer only badges from the factory set (no per-card verify)', async () => {
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          asset_infos: [{ token: { contract_addr: 't1' } }, { token: { contract_addr: 't2' } }],
          contract_addr: 'inFactory',
          liquidity_token: 'lp',
        },
      ],
    })
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [mockIndexerPair('inFactory'), mockIndexerPair('notInFactory')],
      total: 2,
      limit: 20,
      offset: 0,
    })
    renderWithProviders(<PoolPage />)
    expect(await screen.findByText('In router (factory)')).toBeInTheDocument()
    expect(await screen.findByText('Indexer only')).toBeInTheDocument()
  })

  it('filters the current page to factory pairs when the filter is on', async () => {
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          asset_infos: [{ token: { contract_addr: 't1' } }, { token: { contract_addr: 't2' } }],
          contract_addr: 'inFactory',
          liquidity_token: 'lp',
        },
      ],
    })
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      items: [mockIndexerPair('inFactory'), mockIndexerPair('notInFactory')],
      total: 2,
      limit: 20,
      offset: 0,
    })
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />)
    await screen.findByText('In router (factory)')
    const filter = screen.getByRole('checkbox', { name: /Router-known \(factory\) only on this page/i })
    await user.click(filter)
    expect(screen.getAllByText('In router (factory)').length).toBe(1)
    expect(screen.queryByText('Indexer only')).not.toBeInTheDocument()
  })
})
