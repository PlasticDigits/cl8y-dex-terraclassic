import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import PoolPage from './PoolPage'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getPairPaused, getPool } from '@/services/terraclassic/pair'
import { probePairCodeIdFreeze } from '@/services/terraclassic/assetCodeIdFreeze'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerPair } from '@/types'
import { POOL_VOL_HEADER_TITLE } from '@/utils/trailingWindowCopy'

const { CLUNC, CUSTC, UST1 } = vi.hoisted(() => ({
  CLUNC: 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg',
  CUSTC: 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch',
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    WRAPPED_NATIVE_PAIRS: {
      ...actual.WRAPPED_NATIVE_PAIRS,
      [CLUNC]: 'uluna',
      [CUSTC]: 'uusd',
    },
  }
})

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

vi.mock('@/services/terraclassic/assetCodeIdFreeze', () => ({
  probePairCodeIdFreeze: vi.fn().mockResolvedValue({ frozen: false, verdict: 'tradable' }),
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

const { addr, walletSnapshot } = vi.hoisted(() => {
  const addr = 'terra1test00000000000000000000000000000000'
  return {
    addr,
    walletSnapshot: { address: addr as string | null, openWalletModal: vi.fn() },
  }
})

vi.mock('@/hooks/useWallet', () => ({
  useWalletStore: (fn: (s: { address: string | null; openWalletModal: () => void }) => unknown) => fn(walletSnapshot),
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

async function openPoolCardAdvanced(user: ReturnType<typeof userEvent.setup>) {
  const manage = await screen.findByTestId('pool-row-manage')
  if (manage.getAttribute('aria-expanded') !== 'true') {
    await user.click(manage)
  }
  const details = await screen.findByTestId('pool-card-advanced')
  if (!(details as HTMLDetailsElement).open) {
    await user.click(details.querySelector('summary') as HTMLElement)
  }
}

describe('PoolPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    walletSnapshot.address = addr
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
    vi.mocked(getPairPaused).mockResolvedValue({ paused: false })
    vi.mocked(probePairCodeIdFreeze).mockResolvedValue({ frozen: false, verdict: 'tradable' })
    vi.mocked(indexerClient.getTokens).mockResolvedValue([])
    vi.mocked(indexerClient.getPairs).mockResolvedValue(mockGetPairs)
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          asset_infos: [{ token: { contract_addr: 'tokenA' } }, { token: { contract_addr: 'tokenB' } }],
          contract_addr: mockPair.pair_address,
          liquidity_token: mockPair.lp_token,
        },
      ],
    })
    getTokenBalanceMock.mockImplementation(
      async (wallet: string, balanceInfo?: { native_token?: { denom: string }; token?: { contract_addr: string } }) => {
        if (wallet !== addr) return '0'
        if (balanceInfo?.native_token?.denom === 'uluna') {
          return '50000000000000'
        }
        const id = (balanceInfo?.token?.contract_addr ?? balanceInfo?.native_token?.denom ?? '').toLowerCase()
        if (id.includes('lp')) {
          return '2000000000000000000'
        }
        return '2000000'
      }
    )
  })

  it('renders without crashing', async () => {
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(await screen.findByTestId('pool-pairs-table')).toBeTruthy()
  })

  it('renders one-sided add and withdraw cards (GitLab #533)', () => {
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(screen.getByTestId('pool-one-sided-add')).toBeInTheDocument()
    expect(screen.getByTestId('pool-one-sided-withdraw')).toBeInTheDocument()
    expect(screen.getByTestId('pool-one-sided-add-submit')).toBeInTheDocument()
    expect(screen.getByTestId('pool-il-risk-notice')).toBeInTheDocument()
    expect(screen.getByTestId('pool-one-sided-add-amount')).toBeInTheDocument()
    expect(screen.queryByLabelText(/tokenB amount/i)).not.toBeInTheDocument()
    expect(screen.queryByLabelText(/Asset A|Asset B/i)).not.toBeInTheDocument()
    expect(screen.queryByTestId('pool-provide-auto-wrap-a')).not.toBeInTheDocument()
  })

  it('U1 disconnected: pickers + IL visible; CTA is Connect Wallet', () => {
    walletSnapshot.address = null
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(screen.getByTestId('pool-il-risk-notice')).toBeInTheDocument()
    expect(screen.getByLabelText(/^Token$/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/^Pair$/i)).toBeInTheDocument()
    expect(screen.getByTestId('pool-one-sided-add-submit')).toHaveTextContent(/Connect Wallet/i)
    expect(screen.getByTestId('pool-one-sided-withdraw-submit')).toHaveTextContent(/Connect Wallet/i)
  })

  it('U2 connected with zero holdings: empty token and LP states', async () => {
    getTokenBalanceMock.mockResolvedValue('0')
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(await screen.findByTestId('pool-one-sided-add-empty-tokens')).toBeInTheDocument()
    expect(await screen.findByTestId('pool-one-sided-withdraw-empty-lp')).toBeInTheDocument()
  })

  it('U11 how-to no longer requires both tokens (H531-3)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await user.click(screen.getByTestId('pool-lp-howto-open'))
    expect(screen.getByTestId('pool-lp-howto-step-two-sided')).toHaveTextContent(/one token/i)
    expect(screen.getByTestId('pool-lp-howto-step-two-sided')).not.toHaveTextContent(/both assets are required/i)
  })

  it('shows retail LP how-to and still renders Provide / IL when opened (GitLab #531)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    expect(screen.getByTestId('pool-lp-howto')).toBeInTheDocument()
    expect(screen.getByTestId('pool-lp-howto-details')).toBeInTheDocument()
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
    expect(await screen.findByTestId('pool-il-risk-notice')).toBeInTheDocument()
    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)
    expect(await screen.findByTestId('pool-il-risk-notice-advanced')).toBeInTheDocument()
  })

  it('shows impermanent loss notice when provide panel is open (GitLab #366)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    expect(await screen.findByTestId('pool-il-risk-notice')).toBeInTheDocument()

    await openPoolCardAdvanced(user)
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

    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    expect(screen.queryByTestId('pool-provide-pre-submit-summary')).not.toBeInTheDocument()

    const aInput = screen.getByLabelText('tokenA amount')
    await user.type(aInput, '1')

    const summary = await screen.findByTestId('pool-provide-pre-submit-summary')
    expect(summary).toHaveTextContent('Provide Liquidity')
    expect(screen.getByTestId('pool-provide-pre-submit-summary-pair')).toBeInTheDocument()
    expect(screen.getByTestId('pool-provide-pre-submit-summary-amount')).toHaveTextContent('1')
    expect(screen.getByLabelText('tokenB amount')).toHaveValue('2')
    expect(screen.getByTestId('pool-provide-pre-submit-summary-chain')).toBeInTheDocument()

    const submitButtons = screen.getAllByRole('button', { name: /^Provide Liquidity$/i })
    const submit = submitButtons[submitButtons.length - 1]!
    expect(summary.compareDocumentPosition(submit) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('shows withdraw pre-sign summary before submit when LP amount is entered (GitLab #462 / SEC-I05)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })

    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    await openPoolCardAdvanced(user)
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

    await waitFor(() => {
      expect(screen.getAllByRole('button', { name: /^Withdraw Liquidity$/i }).length).toBeGreaterThan(1)
    })
    const submitButtons = screen.getAllByRole('button', { name: /^Withdraw Liquidity$/i })
    const submit = submitButtons[submitButtons.length - 1]!
    expect(summary.compareDocumentPosition(submit) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('add-LP: shows per-asset balance and estimated LP when provide panel is open', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })

    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    expect(await screen.findAllByTestId('pool-add-max-a')).toHaveLength(1)
    expect(await screen.findAllByTestId('pool-add-max-b')).toHaveLength(1)
    const balanceLines = screen.getAllByText(/^Balance:/i)
    expect(balanceLines.length).toBeGreaterThanOrEqual(2)

    const aInput = screen.getByLabelText('tokenA amount')
    await user.type(aInput, '1')

    await waitFor(() => {
      expect(screen.getByText(/Estimated LP:/i)).toBeInTheDocument()
    })
  })

  it('auto-fills B when typing A on non-empty pool (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    await user.type(screen.getByLabelText('tokenA amount'), '1')
    expect(screen.getByLabelText('tokenB amount')).toHaveValue('2')
  })

  it('auto-fills A when typing B with A empty (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    await user.type(screen.getByLabelText('tokenB amount'), '2')
    expect(screen.getByLabelText('tokenA amount')).toHaveValue('1')
  })

  it('Max on A force-syncs B (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    await user.click(await screen.findByTestId('pool-add-max-a'))
    const aVal = (screen.getByLabelText('tokenA amount') as HTMLInputElement).value
    const bVal = (screen.getByLabelText('tokenB amount') as HTMLInputElement).value
    expect(aVal).not.toBe('')
    expect(bVal).not.toBe('')
    expect(Number(bVal) / Number(aVal)).toBeCloseTo(2, 5)
  })

  it('shows ratio warning when user overrides auto-filled B (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const bInput = screen.getByLabelText('tokenB amount')
    await user.type(screen.getByLabelText('tokenA amount'), '1')
    expect(bInput).toHaveValue('2')

    await user.clear(bInput)
    await user.type(bInput, '3')

    expect(screen.getByLabelText('tokenA amount')).toHaveValue('1')
    expect(await screen.findByTestId('pool-provide-ratio-warning')).toBeInTheDocument()
  })

  it('shows withdraw estimated receive preview (#480)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

    await openPoolCardAdvanced(user)
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
    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const aInput = await screen.findByLabelText('tokenA amount')
    await user.clear(aInput)
    await user.type(aInput, '2')
    await user.type(screen.getByLabelText('tokenB amount'), '2')

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
    await openPoolCardAdvanced(user)
    const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
    await user.click(provide[0]!)

    const aInput = await screen.findByLabelText('tokenA amount')
    await user.type(aInput, '1')

    expect(await screen.findByRole('alert')).toHaveTextContent(/allowance A \+ allowance B \+ provide liquidity/)
    expect(screen.getByRole('button', { name: /Not enough LUNC for gas/i })).toBeDisabled()
  })

  it('strips page lectures and shows a sortable table (GitLab #547)', async () => {
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
    expect(await screen.findByTestId('pool-pairs-table')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /Liquidity Pools/i })).not.toBeInTheDocument()
    expect(screen.queryByText(/List source:/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/indexed tokens/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/factory, router graph/i)).not.toBeInTheDocument()
    expect(screen.queryByTestId('pool-fee-discount-eligibility-note')).not.toBeInTheDocument()
    expect(screen.queryByRole('checkbox', { name: /Router-known/i })).not.toBeInTheDocument()
    expect(screen.queryByLabelText(/^Sort$/i)).not.toBeInTheDocument()
  })

  it('shows compact Factory vs Indexer marks from the factory set (no per-row LCD verify)', async () => {
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
    expect(await screen.findByTestId('pool-row-factory')).toHaveTextContent('Factory')
    expect(screen.getByTestId('pool-row-indexer-only')).toHaveTextContent('Indexer')
  })

  it('does not offer a Router-known filter (GitLab #547 AC7)', async () => {
    renderWithProviders(<PoolPage />, { route: '/pool' })
    await screen.findByTestId('pool-pairs-table')
    expect(screen.queryByTestId('pool-filter-router')).not.toBeInTheDocument()
    expect(document.getElementById('pool-filter-router')).toBeNull()
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

      await openPoolCardAdvanced(user)
      const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
      await user.click(provide[0]!)

      expect(await screen.findByTestId('pool-pair-paused-banner')).toHaveTextContent(/paused by governance/i)
      const aInput = await screen.findByLabelText('tokenA amount')
      const bInput = screen.getByLabelText('tokenB amount')
      await user.type(aInput, '1')
      await user.type(bInput, '2')

      expect(screen.getByRole('button', { name: 'Pair is paused' })).toBeDisabled()
    })

    it('disables withdraw liquidity submit when pair is paused', async () => {
      vi.mocked(getPairPaused).mockResolvedValue({ paused: true })
      const user = userEvent.setup()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

      await openPoolCardAdvanced(user)
      const withdrawTabs = await screen.findAllByRole('button', { name: /Withdraw Liquidity/i })
      await user.click(withdrawTabs[0]!)

      expect(await screen.findByTestId('pool-pair-paused-banner')).toHaveTextContent(/paused by governance/i)
      const lpInput = screen.getByLabelText('LP Token Amount')
      await user.type(lpInput, '1')

      expect(screen.getByRole('button', { name: 'Pair is paused' })).toBeDisabled()
    })
  })

  describe('code-id freeze disabled LP CTAs (GitLab #585)', () => {
    it('shows Frozen badge when indexer flags code_id_frozen', async () => {
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        ...mockGetPairs,
        items: [{ ...mockPair, code_id_frozen: true }],
      })
      renderWithProviders(<PoolPage />, { route: '/pool' })
      expect(await screen.findByTestId('pool-row-code-id-frozen')).toHaveTextContent(/Frozen/i)
    })

    it('disables provide liquidity submit when pair is code-id frozen', async () => {
      vi.mocked(probePairCodeIdFreeze).mockResolvedValue({ frozen: true, verdict: 'frozen' })
      const user = userEvent.setup()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

      await openPoolCardAdvanced(user)
      const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
      await user.click(provide[0]!)

      expect(await screen.findByTestId('pool-pair-code-id-frozen-banner')).toHaveTextContent(/quotes can still appear/i)
      const aInput = await screen.findByLabelText('tokenA amount')
      const bInput = screen.getByLabelText('tokenB amount')
      await user.type(aInput, '1')
      await user.type(bInput, '2')

      expect(screen.getByRole('button', { name: 'Market frozen' })).toBeDisabled()
    })

    it('disables withdraw liquidity submit when pair is code-id frozen', async () => {
      vi.mocked(probePairCodeIdFreeze).mockResolvedValue({ frozen: true, verdict: 'frozen' })
      const user = userEvent.setup()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())

      await openPoolCardAdvanced(user)
      const withdrawTabs = await screen.findAllByRole('button', { name: /Withdraw Liquidity/i })
      await user.click(withdrawTabs[0]!)

      expect(await screen.findByTestId('pool-pair-code-id-frozen-banner')).toHaveTextContent(/quotes can still appear/i)
      const lpInput = screen.getByLabelText('LP Token Amount')
      await user.type(lpInput, '1')

      expect(screen.getByRole('button', { name: 'Market frozen' })).toBeDisabled()
    })
  })

  describe('trading blacklist UX (GitLab #388 / SEC-E01)', () => {
    async function openProvidePanel(user: ReturnType<typeof userEvent.setup>) {
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
      await openPoolCardAdvanced(user)
      const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
      await user.click(provide[0]!)
      await user.type(await screen.findByLabelText('tokenA amount'), '1')
    }

    async function openWithdrawPanel(user: ReturnType<typeof userEvent.setup>) {
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
      await openPoolCardAdvanced(user)
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

  describe('token identity (GitLab #541)', () => {
    const PAIR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'
    const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
    const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
    const LP = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

    function mockIdentityPair() {
      const pair: IndexerPair = {
        pair_address: PAIR,
        asset_0: { symbol: 'UST1', contract_addr: UST1, denom: null, decimals: 6 },
        asset_1: { symbol: 'cUSTC', contract_addr: CUSTC, denom: null, decimals: 6 },
        lp_token: LP,
        fee_bps: 30,
        is_active: true,
      }
      vi.mocked(indexerClient.getPairs).mockResolvedValue({ total: 1, items: [pair], limit: 20, offset: 0 })
      vi.mocked(getAllPairsPaginated).mockResolvedValue({
        pairs: [
          {
            asset_infos: [{ token: { contract_addr: UST1 } }, { token: { contract_addr: CUSTC } }],
            contract_addr: PAIR,
            liquidity_token: LP,
          },
        ],
      })
    }

    it('U1: shows both legs + pair chip; LP AddressRow still on withdraw', async () => {
      const user = userEvent.setup()
      mockIdentityPair()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      expect(await screen.findByTestId('pair-token-links')).toBeInTheDocument()
      expect(screen.getByTestId('token-identity-base')).toHaveAttribute('data-identity-payload', UST1)
      expect(screen.getByTestId('token-identity-quote')).toHaveAttribute('data-identity-payload', CUSTC)
      expect(screen.getByTestId('token-identity-pair')).toBeInTheDocument()
      expect(screen.getByTestId('token-identity-base-explorer')).toHaveAttribute('rel', 'noopener noreferrer')

      await openPoolCardAdvanced(user)
      const withdrawTabs = await screen.findAllByRole('button', { name: /Withdraw Liquidity/i })
      await user.click(withdrawTabs[0]!)
      expect(await screen.findByTestId('pool-lp-token-address-row')).toBeInTheDocument()
    })

    it('U2: native wrap option label does not get an explorer href', async () => {
      mockIdentityPair()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await screen.findByTestId('pair-token-links')
      expect(screen.queryByRole('link', { name: /uluna/i })).not.toBeInTheDocument()
    })
  })

  describe('sortable table + catalog default (GitLab #547)', () => {
    const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
    const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
    const EMBER = 'terra1ember00000000000000000000000000000001'
    const CORAL = 'terra1coral00000000000000000000000000000002'
    const UST1_PAIR = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'
    const GEM_PAIR = 'terra1gempair0000000000000000000000000000001'

    function catalogPairs(): IndexerPair[] {
      return [
        {
          pair_address: GEM_PAIR,
          asset_0: { symbol: 'EMBER', contract_addr: EMBER, denom: null, decimals: 6 },
          asset_1: { symbol: 'CORAL', contract_addr: CORAL, denom: null, decimals: 6 },
          lp_token: 'lp-gem',
          fee_bps: 30,
          is_active: true,
          volume_quote_24h: '999999',
        },
        {
          pair_address: UST1_PAIR,
          asset_0: { symbol: 'UST1', contract_addr: UST1, denom: null, decimals: 6 },
          asset_1: { symbol: 'cUSTC', contract_addr: CUSTC, denom: null, decimals: 6 },
          lp_token: 'lp-ust1',
          fee_bps: 30,
          is_active: true,
          volume_quote_24h: '1',
        },
      ]
    }

    it('S1: default catalog ranks UST1 ahead of gems', async () => {
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: catalogPairs(),
        total: 2,
        limit: 500,
        offset: 0,
      })
      renderWithProviders(<PoolPage />, { route: '/pool' })
      const links = await screen.findAllByTestId('pool-row-charts')
      expect(links[0]).toHaveAttribute('href', `/charts/${UST1_PAIR}`)
      expect(links[1]).toHaveAttribute('href', `/charts/${GEM_PAIR}`)
      expect(indexerClient.getPairs).toHaveBeenCalledWith(
        expect.objectContaining({ limit: 500, sort: 'volume_24h', order: 'desc' })
      )
    })

    it('S2/S3: Vol header calls indexer volume_24h and toggles order', async () => {
      const user = userEvent.setup()
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: catalogPairs(),
        total: 2,
        limit: 20,
        offset: 0,
      })
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await screen.findByTestId('pool-pairs-table')
      expect(screen.getByTestId('pool-sort-vol')).toHaveAttribute('title', POOL_VOL_HEADER_TITLE)
      expect(screen.getByTestId('pool-sort-vol')).toHaveTextContent(/Vol/)
      await user.click(screen.getByTestId('pool-sort-vol'))
      await waitFor(() =>
        expect(indexerClient.getPairs).toHaveBeenCalledWith(
          expect.objectContaining({ sort: 'volume_24h', order: 'desc', limit: 20 })
        )
      )
      expect(screen.getByTestId('pool-sort-vol').closest('th')).toHaveAttribute('aria-sort', 'descending')
      await user.click(screen.getByTestId('pool-sort-vol'))
      await waitFor(() =>
        expect(indexerClient.getPairs).toHaveBeenCalledWith(
          expect.objectContaining({ sort: 'volume_24h', order: 'asc', limit: 20 })
        )
      )
    })

    it('S4: Pair header sorts by symbol without catalog overlay', async () => {
      const user = userEvent.setup()
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: catalogPairs(),
        total: 2,
        limit: 20,
        offset: 0,
      })
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await user.click(await screen.findByTestId('pool-sort-pair'))
      await waitFor(() =>
        expect(indexerClient.getPairs).toHaveBeenCalledWith(
          expect.objectContaining({ sort: 'symbol', order: 'asc', limit: 20 })
        )
      )
      expect(screen.getByTestId('pool-sort-pair').closest('th')).toHaveAttribute('aria-sort', 'ascending')
      expect(screen.getByTestId('pool-sort-vol').closest('th')).toHaveAttribute('aria-sort', 'none')
    })

    it('H1: Charts link is same-origin /charts/:pairAddr', async () => {
      vi.mocked(indexerClient.getPairs).mockResolvedValue({
        items: catalogPairs(),
        total: 2,
        limit: 500,
        offset: 0,
      })
      renderWithProviders(<PoolPage />, { route: '/pool' })
      const link = await screen.findAllByTestId('pool-row-charts')
      expect(link[0]).toHaveAttribute('href', `/charts/${UST1_PAIR}`)
    })
  })

  describe('provide labels + wrap default (GitLab #661)', () => {
    const PAIR = 'terra1wrapair000000000000000000000000000001'
    const LP = 'terra1wraplp0000000000000000000000000000001'

    function mockWrapPair(asset0: string, asset1: string) {
      const pair: IndexerPair = {
        pair_address: PAIR,
        asset_0: { symbol: 'x', contract_addr: asset0, denom: null, decimals: 6 },
        asset_1: { symbol: 'y', contract_addr: asset1, denom: null, decimals: 6 },
        lp_token: LP,
        fee_bps: 30,
        is_active: true,
      }
      vi.mocked(indexerClient.getPairs).mockResolvedValue({ total: 1, items: [pair], limit: 20, offset: 0 })
      vi.mocked(getAllPairsPaginated).mockResolvedValue({
        pairs: [
          {
            asset_infos: [{ token: { contract_addr: asset0 } }, { token: { contract_addr: asset1 } }],
            contract_addr: PAIR,
            liquidity_token: LP,
          },
        ],
      })
      vi.mocked(getPool).mockResolvedValue({
        assets: [
          { info: { token: { contract_addr: asset0 } }, amount: '1000000' },
          { info: { token: { contract_addr: asset1 } }, amount: '2000000' },
        ],
        total_share: '2000000',
      })
    }

    async function openProvide(user: ReturnType<typeof userEvent.setup>) {
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
      await openPoolCardAdvanced(user)
      const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
      await user.click(provide[0]!)
    }

    it('W1/L2: cLUNC leg wrap checkbox is checked; label is native LUNC', async () => {
      const user = userEvent.setup()
      mockWrapPair(CLUNC, UST1)
      await openProvide(user)

      const wrapA = await screen.findByTestId('pool-provide-auto-wrap-a')
      expect(wrapA).toBeChecked()
      expect(screen.queryByTestId('pool-provide-auto-wrap-b')).not.toBeInTheDocument()
      expect(screen.getByTestId('pool-provide-field-label-a')).toHaveTextContent('Terra Luna Classic (LUNC)')
      expect(screen.getByLabelText('LUNC amount')).toBeInTheDocument()
      expect(screen.getByTestId('pool-provide-field-label-b')).toHaveTextContent('UST1')
      expect(screen.getByLabelText('UST1 amount')).toBeInTheDocument()
      expect(screen.queryByText(/Asset A/i)).not.toBeInTheDocument()
      expect(screen.queryByText(/Asset B/i)).not.toBeInTheDocument()

      await waitFor(() => {
        expect(getTokenBalanceMock).toHaveBeenCalledWith(
          addr,
          expect.objectContaining({ native_token: { denom: 'uluna' } })
        )
      })
    })

    it('L3/W4: uncheck wrap switches label and balance to cLUNC', async () => {
      const user = userEvent.setup()
      mockWrapPair(CLUNC, UST1)
      await openProvide(user)

      await user.click(await screen.findByTestId('pool-provide-auto-wrap-a'))
      expect(screen.getByTestId('pool-provide-auto-wrap-a')).not.toBeChecked()
      expect(screen.getByTestId('pool-provide-field-label-a')).toHaveTextContent('Wrapped Luna Classic (cLUNC)')
      expect(screen.getByLabelText('cLUNC amount')).toBeInTheDocument()

      await waitFor(() => {
        expect(getTokenBalanceMock).toHaveBeenCalledWith(
          addr,
          expect.objectContaining({ token: { contract_addr: CLUNC } })
        )
      })
    })

    it('W2: non-wrap pair has no auto-wrap checkbox', async () => {
      const user = userEvent.setup()
      renderWithProviders(<PoolPage />, { route: '/pool' })
      await waitFor(() => expect(indexerClient.getPairs).toHaveBeenCalled())
      await openPoolCardAdvanced(user)
      const provide = await screen.findAllByRole('button', { name: /Provide Liquidity/i })
      await user.click(provide[0]!)

      expect(screen.queryByTestId('pool-provide-auto-wrap-a')).not.toBeInTheDocument()
      expect(screen.queryByTestId('pool-provide-auto-wrap-b')).not.toBeInTheDocument()
      expect(screen.getByLabelText('tokenA amount')).toBeInTheDocument()
      expect(screen.getByLabelText('tokenB amount')).toBeInTheDocument()
    })

    it('AC8: dual wrap pair defaults both checkboxes on independently', async () => {
      const user = userEvent.setup()
      mockWrapPair(CLUNC, CUSTC)
      await openProvide(user)

      expect(await screen.findByTestId('pool-provide-auto-wrap-a')).toBeChecked()
      expect(screen.getByTestId('pool-provide-auto-wrap-b')).toBeChecked()
      expect(screen.getByLabelText('LUNC amount')).toBeInTheDocument()
      expect(screen.getByLabelText('USTC amount')).toBeInTheDocument()

      await user.click(screen.getByTestId('pool-provide-auto-wrap-b'))
      expect(screen.getByTestId('pool-provide-auto-wrap-a')).toBeChecked()
      expect(screen.getByTestId('pool-provide-auto-wrap-b')).not.toBeChecked()
      expect(screen.getByLabelText('cUSTC amount')).toBeInTheDocument()
    })
  })
})
