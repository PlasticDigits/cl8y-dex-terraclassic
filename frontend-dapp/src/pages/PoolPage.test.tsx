import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import PoolPage from './PoolPage'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerPair } from '@/types'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getPairs: vi.fn(),
    getTokens: vi.fn(),
  }
})

const mockPair: IndexerPair = {
  pair_address: 'terra1pair0000000000000000000000000000000',
  asset_0: { symbol: 'TKA', contract_addr: 'tokenA', denom: null, decimals: 6 },
  asset_1: { symbol: 'TKB', contract_addr: 'tokenB', denom: null, decimals: 6 },
  lp_token: 'lptoken1',
  fee_bps: 30,
  is_active: true,
}

const mockGetPairs = {
  total: 1,
  items: [mockPair],
  limit: 20,
  offset: 0,
}

vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn().mockResolvedValue({ pairs: [] }),
}))

const getTokenBalanceMock = vi.fn()

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: (...args: unknown[]) => getTokenBalanceMock(...args),
  verifyPairInFactory: vi.fn().mockResolvedValue(true),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: 'tokenA' } }, amount: '1000000' },
      { info: { token: { contract_addr: 'tokenB' } }, amount: '2000000' },
    ],
    total_share: '2000000',
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

const addr = 'terra1test00000000000000000000000000000000'

vi.mock('@/hooks/useWallet', () => ({
  useWalletStore: (fn: (s: { address: string | null }) => unknown) => fn({ address: addr }),
}))

describe('PoolPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(indexerClient.getTokens).mockResolvedValue([])
    vi.mocked(indexerClient.getPairs).mockResolvedValue(mockGetPairs)
    getTokenBalanceMock.mockImplementation(async (wallet: string) => {
      if (wallet !== addr) return '0'
      return '2000000'
    })
  })

  it('renders without crashing', () => {
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(screen.getByText(/liquidity pools/i)).toBeTruthy()
  })

  it('add-LP: shows per-asset balance and estimated LP when provide panel is open', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })

    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    expect(await screen.findAllByLabelText(/Balance and actions for/i)).toHaveLength(2)
    const balanceLines = screen.getAllByText(/^Balance:/i)
    expect(balanceLines.length).toBeGreaterThanOrEqual(2)

    const aInput = screen.getByLabelText('Asset A amount')
    const bInput = screen.getByLabelText('Asset B amount')
    await user.type(aInput, '1')
    await user.type(bInput, '2')

    await waitFor(() => {
      expect(screen.getByText(/Estimated LP:/i)).toBeInTheDocument()
    })
  })

  it('disables provide when amount exceeds balance', async () => {
    const user = userEvent.setup()
    getTokenBalanceMock.mockImplementation(async (wallet) => (wallet === addr ? '1000000' : '0'))

    renderWithProviders(<PoolPage />, { route: '/pool' })
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const aInput = await screen.findByLabelText('Asset A amount')
    const bInput = screen.getByLabelText('Asset B amount')
    await user.clear(aInput)
    await user.type(aInput, '2')
    await user.clear(bInput)
    await user.type(bInput, '2')

    const submit = screen.getByRole('button', { name: /Insufficient balance/i })
    expect(submit).toBeDisabled()
    expect(screen.getAllByText(/Exceeds wallet balance/i).length).toBeGreaterThanOrEqual(1)
  })
})
