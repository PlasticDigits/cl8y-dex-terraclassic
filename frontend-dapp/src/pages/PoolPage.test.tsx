import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import PoolPage from './PoolPage'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getPairPaused } from '@/services/terraclassic/pair'
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
  getPairPaused: vi.fn().mockResolvedValue({ paused: false }),
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
  getRegistration: vi.fn().mockResolvedValue({ registered: false, tier_id: null, tier: null }),
}))

vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn().mockResolvedValue(null),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

vi.mock('@/hooks/useTradingBlacklist', () => ({
  useTradingBlacklist: vi.fn(),
}))

const addr = 'terra1test00000000000000000000000000000000'

vi.mock('@/hooks/useWallet', () => ({
  useWalletStore: (fn: (s: { address: string | null }) => unknown) => fn({ address: addr }),
}))

import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import {
  TRADING_BLACKLIST_ALLOWED,
  pairBlacklistedResponse,
  tokenBlacklistedResponse,
  tradingBlacklistHookResult,
  walletBlacklistedResponse,
} from '@/test/tradingBlacklistMocks'
import { describeTradingBlacklistBlock } from '@/services/terraclassic/blacklist'

const mockIndexerPair = (pairAddr: string): IndexerPair => ({
  pair_address: pairAddr,
  asset_0: { symbol: 'A', contract_addr: 'tokenA', denom: null, decimals: 6 },
  asset_1: { symbol: 'B', contract_addr: 'tokenB', denom: null, decimals: 6 },
  lp_token: 'lp1',
  fee_bps: 30,
  is_active: true,
})

describe('PoolPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
    vi.mocked(getPairPaused).mockResolvedValue({ paused: false })
    vi.mocked(indexerClient.getTokens).mockResolvedValue([])
    vi.mocked(indexerClient.getPairs).mockResolvedValue(mockGetPairs)
    vi.mocked(getAllPairsPaginated).mockResolvedValue({ pairs: [] })
    getTokenBalanceMock.mockImplementation(
      async (wallet: string, balanceInfo?: { native_token?: { denom: string } }) => {
        if (wallet !== addr) return '0'
        if (balanceInfo?.native_token?.denom === 'uluna') {
          return '50000000000000'
        }
        return '2000000'
      }
    )
  })

  it('renders without crashing', () => {
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(screen.getByText(/liquidity pools/i)).toBeTruthy()
  })

  it('shows retail LP how-to and still renders Provide / IL when opened (GitLab #531)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(screen.getByTestId('pool-lp-howto')).toBeInTheDocument()
    expect(screen.getByTestId('pool-lp-howto-details')).toBeInTheDocument()
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)
    expect(await screen.findByTestId('pool-il-risk-notice')).toBeInTheDocument()
  })

  it('shows impermanent loss notice when provide panel is open (GitLab #366)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    expect(screen.queryByTestId('pool-il-risk-notice')).not.toBeInTheDocument()

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const notice = await screen.findByTestId('pool-il-risk-notice')
    expect(notice).toHaveTextContent(/Impermanent loss risk/i)
    expect(notice).toHaveTextContent(/diverge from simply holding/i)
    expect(within(notice).getByRole('link', { name: /Learn more/i })).toHaveAttribute(
      'href',
      'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/frontend.md#pool-lp-risk-disclosure'
    )
  })

  it('shows provide pre-sign summary before submit when both amounts are entered (GitLab #462 / SEC-I05)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })

    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    expect(screen.queryByTestId('pool-provide-pre-submit-summary')).not.toBeInTheDocument()

    const aInput = screen.getByLabelText('Asset A amount')
    await user.type(aInput, '1')

    const summary = await screen.findByTestId('pool-provide-pre-submit-summary')
    expect(summary).toHaveTextContent('Provide Liquidity')
    expect(screen.getByTestId('pool-provide-pre-submit-summary-pair')).toBeInTheDocument()
    expect(screen.getByTestId('pool-provide-pre-submit-summary-amount')).toHaveTextContent('1')
    expect(screen.getByLabelText('Asset B amount')).toHaveValue('2')
    expect(screen.getByTestId('pool-provide-pre-submit-summary-chain')).toBeInTheDocument()

    const submitButtons = screen.getAllByRole('button', { name: /^Provide Liquidity$/i })
    const submit = submitButtons[submitButtons.length - 1]!
    expect(summary.compareDocumentPosition(submit) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('shows withdraw pre-sign summary before submit when LP amount is entered (GitLab #462 / SEC-I05)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })

    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const withdrawTabs = await screen.findAllByRole('button', { name: /Withdraw Liquidity/i })
    await user.click(withdrawTabs[0]!)

    expect(screen.queryByTestId('pool-withdraw-pre-submit-summary')).not.toBeInTheDocument()

    const lpInput = screen.getByLabelText('LP Token Amount')
    await user.type(lpInput, '1.5')

    const summary = await screen.findByTestId('pool-withdraw-pre-submit-summary')
    expect(summary).toHaveTextContent('Withdraw Liquidity')
    expect(screen.getByTestId('pool-withdraw-pre-submit-summary-amount')).toHaveTextContent('1.5 LP')
    expect(screen.getByTestId('pool-withdraw-pre-submit-summary-amount')).toHaveTextContent('tokenA')
    expect(screen.getByTestId('pool-withdraw-pre-submit-summary-amount')).toHaveTextContent('tokenB')
    expect(screen.getByTestId('pool-withdraw-pre-submit-summary-chain')).toBeInTheDocument()

    const submitButtons = screen.getAllByRole('button', { name: /^Withdraw Liquidity$/i })
    const submit = submitButtons[submitButtons.length - 1]!
    expect(summary.compareDocumentPosition(submit) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('add-LP: shows per-asset balance and estimated LP when provide panel is open', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })

    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    expect(await screen.findAllByTestId('pool-add-max-a')).toHaveLength(1)
    expect(await screen.findAllByTestId('pool-add-max-b')).toHaveLength(1)
    const balanceLines = screen.getAllByText(/^Balance:/i)
    expect(balanceLines.length).toBeGreaterThanOrEqual(2)

    const aInput = screen.getByLabelText('Asset A amount')
    await user.type(aInput, '1')

    await waitFor(() => {
      expect(screen.getByText(/Estimated LP:/i)).toBeInTheDocument()
    })
  })

  it('auto-fills B when typing A on non-empty pool (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    await user.type(screen.getByLabelText('Asset A amount'), '1')
    expect(screen.getByLabelText('Asset B amount')).toHaveValue('2')
  })

  it('auto-fills A when typing B with A empty (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    await user.type(screen.getByLabelText('Asset B amount'), '2')
    expect(screen.getByLabelText('Asset A amount')).toHaveValue('1')
  })

  it('Max on A force-syncs B (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    await user.click(await screen.findByTestId('pool-add-max-a'))
    const aVal = (screen.getByLabelText('Asset A amount') as HTMLInputElement).value
    const bVal = (screen.getByLabelText('Asset B amount') as HTMLInputElement).value
    expect(aVal).not.toBe('')
    expect(bVal).not.toBe('')
    expect(Number(bVal) / Number(aVal)).toBeCloseTo(2, 5)
  })

  it('shows ratio warning when user overrides auto-filled B (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const bInput = screen.getByLabelText('Asset B amount')
    await user.type(screen.getByLabelText('Asset A amount'), '1')
    expect(bInput).toHaveValue('2')

    await user.clear(bInput)
    await user.type(bInput, '3')

    expect(screen.getByLabelText('Asset A amount')).toHaveValue('1')
    expect(await screen.findByTestId('pool-provide-ratio-warning')).toBeInTheDocument()
  })

  it('shows withdraw estimated receive preview (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    const withdrawTabs = await screen.findAllByRole('button', { name: /Withdraw Liquidity/i })
    await user.click(withdrawTabs[0]!)

    await user.type(screen.getByLabelText('LP Token Amount'), '1')

    const preview = await screen.findByTestId('pool-withdraw-estimated-receive')
    expect(preview).toHaveTextContent(/Expected receive/i)
    expect(preview).toHaveTextContent(/tokenA/)
    expect(preview).toHaveTextContent(/tokenB/)
    expect(screen.getByTestId('pool-withdraw-minimum-receive')).toBeInTheDocument()
  })

  it('disables provide when amount exceeds balance', async () => {
    const user = userEvent.setup()
    getTokenBalanceMock.mockImplementation(async (wallet) => (wallet === addr ? '1000000' : '0'))

    renderWithProviders(<PoolPage />, { route: '/pool' })
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const aInput = await screen.findByLabelText('Asset A amount')
    await user.clear(aInput)
    await user.type(aInput, '2')
    await user.type(screen.getByLabelText('Asset B amount'), '2')

    const submit = screen.getByRole('button', { name: /Insufficient balance/i })
    expect(submit).toBeDisabled()
    expect(screen.getAllByText(/Exceeds wallet balance/i).length).toBeGreaterThanOrEqual(1)
  })

  it('disables provide when LUNC is below the three-tx CW20 path gas floor (GitLab #147)', async () => {
    const user = userEvent.setup()
    getTokenBalanceMock.mockImplementation(
      async (wallet: string, balanceInfo?: { native_token?: { denom: string } }) => {
        if (wallet !== addr) return '0'
        if (balanceInfo?.native_token?.denom === 'uluna') {
          return '1000000'
        }
        return '2000000'
      }
    )

    renderWithProviders(<PoolPage />, { route: '/pool' })
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const aInput = await screen.findByLabelText('Asset A amount')
    await user.type(aInput, '1')

    expect(await screen.findByRole('alert')).toHaveTextContent(/allowance A \+ allowance B \+ provide liquidity/)
    expect(screen.getByRole('button', { name: /Not enough LUNC for gas/i })).toBeDisabled()
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
    renderWithProviders(<PoolPage />, { route: '/pool' })
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
    renderWithProviders(<PoolPage />, { route: '/pool' })
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
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await screen.findByText('In router (factory)')
    const filter = screen.getByRole('checkbox', { name: /Router-known \(factory\) only on this page/i })
    await user.click(filter)
    expect(screen.getAllByText('In router (factory)').length).toBe(1)
    expect(screen.queryByText('Indexer only')).not.toBeInTheDocument()
  })

  it('shows retail market-data banner when pair list fails with transport error (GitLab #215)', async () => {
    vi.mocked(indexerClient.getPairs).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<PoolPage />, { route: '/pool' })
    const banner = await screen.findByTestId('pool-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })

  describe('pair pause disabled LP CTAs (SEC-B05 / GitLab #395)', () => {
    it('disables provide liquidity submit when pair is paused', async () => {
      vi.mocked(getPairPaused).mockResolvedValue({ paused: true })
      const user = userEvent.setup()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

      const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
      await user.click(provide[0]!)

      expect(await screen.findByTestId('pool-pair-paused-banner')).toHaveTextContent(/paused by governance/i)
      const aInput = await screen.findByLabelText('Asset A amount')
      const bInput = screen.getByLabelText('Asset B amount')
      await user.type(aInput, '1')
      await user.type(bInput, '2')

      expect(screen.getByRole('button', { name: 'Pair is paused' })).toBeDisabled()
    })

    it('disables withdraw liquidity submit when pair is paused', async () => {
      vi.mocked(getPairPaused).mockResolvedValue({ paused: true })
      const user = userEvent.setup()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

      const withdrawTabs = await screen.findAllByRole('button', { name: /Withdraw Liquidity/i })
      await user.click(withdrawTabs[0]!)

      expect(await screen.findByTestId('pool-pair-paused-banner')).toHaveTextContent(/paused by governance/i)
      const lpInput = screen.getByLabelText('LP Token Amount')
      await user.type(lpInput, '1')

      expect(screen.getByRole('button', { name: 'Pair is paused' })).toBeDisabled()
    })
  })

  describe('trading blacklist UX (GitLab #388 / SEC-E01)', () => {
    async function openProvidePanel(user: ReturnType<typeof userEvent.setup>) {
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
      const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
      await user.click(provide[0]!)
      await user.type(await screen.findByLabelText('Asset A amount'), '1')
    }

    async function openWithdrawPanel(user: ReturnType<typeof userEvent.setup>) {
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
      const withdrawTabs = await screen.findAllByRole('button', { name: /Withdraw Liquidity/i })
      await user.click(withdrawTabs[0]!)
      await user.type(screen.getByLabelText('LP Token Amount'), '1')
    }

    it.each([
      ['wallet', walletBlacklistedResponse()],
      ['pair', pairBlacklistedResponse(mockPair.pair_address)],
      ['token', tokenBlacklistedResponse('tokenA')],
    ] as const)('shows %s blacklist alert and disables provide liquidity CTA', async (_variant, resp) => {
      const user = userEvent.setup()
      vi.mocked(useTradingBlacklist).mockReturnValue(tradingBlacklistHookResult(resp))
      await openProvidePanel(user)

      const alert = await screen.findByRole('alert')
      expect(alert).toHaveTextContent(describeTradingBlacklistBlock(resp))
      expect(screen.getByRole('button', { name: 'Trading restricted' })).toBeDisabled()
    })

    it.each([
      ['wallet', walletBlacklistedResponse()],
      ['pair', pairBlacklistedResponse(mockPair.pair_address)],
      ['token', tokenBlacklistedResponse('tokenA')],
    ] as const)('shows %s blacklist alert and disables withdraw liquidity CTA', async (_variant, resp) => {
      const user = userEvent.setup()
      vi.mocked(useTradingBlacklist).mockReturnValue(tradingBlacklistHookResult(resp))
      await openWithdrawPanel(user)

      const alert = await screen.findByRole('alert')
      expect(alert).toHaveTextContent(describeTradingBlacklistBlock(resp))
      expect(screen.getByRole('button', { name: 'Trading restricted' })).toBeDisabled()
    })
  })
})
