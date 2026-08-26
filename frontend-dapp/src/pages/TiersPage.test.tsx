import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'
import type { RegistrationResponse, TierEntry } from '@/types'

const { mockFeeDiscountAddr } = vi.hoisted(() => ({
  mockFeeDiscountAddr: { current: 'terra1feediscount000000000000000000000000000' },
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    get FEE_DISCOUNT_CONTRACT_ADDRESS() {
      return mockFeeDiscountAddr.current
    },
    CL8Y_TOKEN_ADDRESS: 'terra1cl8y0000000000000000000000000000000000',
  }
})

vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTiers: vi.fn(),
  getRegistration: vi.fn(),
  register: vi.fn(),
  deregister: vi.fn(),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getFactoryConfig: vi.fn(),
}))

import TiersPage from './TiersPage'
import { getTiers, getRegistration, register, deregister } from '@/services/terraclassic/feeDiscount'
import { getFactoryConfig } from '@/services/terraclassic/settings'

const WALLET = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

/** Canonical 1–9 plus governance 0/255. Numbers stay in docs/reference/fee-discount-tiers.md. */
const FIXTURE_TIERS: TierEntry[] = [
  {
    tier_id: 0,
    tier: { min_cl8y_balance: '0', discount_bps: 10000, limit_discount_bps: 10000, governance_only: true },
  },
  {
    tier_id: 1,
    tier: {
      min_cl8y_balance: '1000000000000000000',
      discount_bps: 250,
      limit_discount_bps: 1000,
      governance_only: false,
    },
  },
  {
    tier_id: 2,
    tier: {
      min_cl8y_balance: '5000000000000000000',
      discount_bps: 1000,
      limit_discount_bps: 2000,
      governance_only: false,
    },
  },
  {
    tier_id: 3,
    tier: {
      min_cl8y_balance: '20000000000000000000',
      discount_bps: 2000,
      limit_discount_bps: 3500,
      governance_only: false,
    },
  },
  {
    tier_id: 4,
    tier: {
      min_cl8y_balance: '75000000000000000000',
      discount_bps: 3500,
      limit_discount_bps: 5000,
      governance_only: false,
    },
  },
  {
    tier_id: 5,
    tier: {
      min_cl8y_balance: '200000000000000000000',
      discount_bps: 5000,
      limit_discount_bps: 6000,
      governance_only: false,
    },
  },
  {
    tier_id: 6,
    tier: {
      min_cl8y_balance: '500000000000000000000',
      discount_bps: 6000,
      limit_discount_bps: 7500,
      governance_only: false,
    },
  },
  {
    tier_id: 7,
    tier: {
      min_cl8y_balance: '1500000000000000000000',
      discount_bps: 7500,
      limit_discount_bps: 8500,
      governance_only: false,
    },
  },
  {
    tier_id: 8,
    tier: {
      min_cl8y_balance: '3500000000000000000000',
      discount_bps: 8500,
      limit_discount_bps: 9500,
      governance_only: false,
    },
  },
  {
    tier_id: 9,
    tier: {
      min_cl8y_balance: '7500000000000000000000',
      discount_bps: 9500,
      limit_discount_bps: 10000,
      governance_only: false,
    },
  },
  {
    tier_id: 255,
    tier: { min_cl8y_balance: '0', discount_bps: 0, limit_discount_bps: 0, governance_only: true },
  },
]

const UNREGISTERED: RegistrationResponse = { registered: false, tier_id: null, tier: null }

function registeredAt(tierId: number): RegistrationResponse {
  const entry = FIXTURE_TIERS.find((t) => t.tier_id === tierId)
  if (!entry) throw new Error(`missing fixture tier ${tierId}`)
  return { registered: true, tier_id: tierId, tier: entry.tier }
}

async function renderTiers() {
  renderWithProviders(<TiersPage />, { route: '/tiers' })
  await waitFor(() => {
    expect(screen.queryByText(/Loading tiers/i)).not.toBeInTheDocument()
  })
}

describe('TiersPage (#651)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockFeeDiscountAddr.current = 'terra1feediscount000000000000000000000000000'
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.mocked(getTiers).mockResolvedValue(FIXTURE_TIERS)
    vi.mocked(getRegistration).mockResolvedValue(UNREGISTERED)
    vi.mocked(register).mockResolvedValue('register-tx')
    vi.mocked(deregister).mockResolvedValue('deregister-tx')
    vi.mocked(getFactoryConfig).mockResolvedValue({
      governance: WALLET,
      treasury: WALLET,
      default_fee_bps: 180,
      pair_code_id: 1,
      lp_token_code_id: 2,
      pair_creation_fee_uluna: '0',
      discount_registry: 'terra1feediscount000000000000000000000000000',
    })
  })

  it('disconnected: connect banner, no Your Status, no Register, hold phrases intact', async () => {
    await renderTiers()

    expect(screen.getByTestId('tiers-connect-banner')).toHaveTextContent(/Connect your wallet/i)
    expect(screen.queryByTestId('tiers-your-status')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Register$/i })).not.toBeInTheDocument()
    expect(screen.queryByTestId('register-tier-1')).not.toBeInTheDocument()

    const hold1 = screen.getByTestId('tier-hold-1')
    expect(hold1).toHaveTextContent('Hold 1 CL8Y')
    expect(hold1).toHaveClass('whitespace-nowrap')
    expect(hold1.childElementCount).toBe(0)

    const hold2 = screen.getByTestId('tier-hold-2')
    expect(hold2).toHaveTextContent('Hold 5 CL8Y')
    expect(hold2).toHaveClass('whitespace-nowrap')

    const hold9 = screen.getByTestId('tier-hold-9')
    expect(hold9).toHaveTextContent('Hold 7.5K CL8Y')
    expect(hold9.className).not.toMatch(/truncate|ellipsis/)

    expect(screen.queryByTestId('tier-card-0')).not.toBeInTheDocument()
    expect(screen.queryByTestId('tier-card-255')).not.toBeInTheDocument()
    expect(screen.queryByText(/Market Maker/i)).not.toBeInTheDocument()
  })

  it('disconnected: fee cluster phrases stay nowrap and there is no reserved Register slot', async () => {
    await renderTiers()

    const card = screen.getByTestId('tier-card-1')
    expect(card.className).not.toMatch(/\bw-28\b/)
    expect(within(card).queryByRole('button')).not.toBeInTheDocument()

    const cluster = screen.getByTestId('tier-fee-cluster-1')
    expect(cluster).toHaveTextContent('2.5%')
    expect(cluster).toHaveTextContent(/fee discount/i)
    expect(cluster).toHaveTextContent('1.75%')
    expect(cluster).toHaveTextContent(/eff\. fee\*/i)
    expect(cluster.querySelectorAll('.whitespace-nowrap').length).toBeGreaterThanOrEqual(4)
  })

  it('connected unregistered: Your Status + Register on tiers 1–9, Deregister absent', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    await renderTiers()

    expect(screen.getByTestId('tiers-your-status')).toHaveTextContent(/Not registered/i)
    expect(screen.queryByTestId('tiers-deregister')).not.toBeInTheDocument()
    expect(screen.queryByTestId('tiers-connect-banner')).not.toBeInTheDocument()

    for (let id = 1; id <= 9; id++) {
      expect(screen.getByTestId(`register-tier-${id}`)).toBeEnabled()
    }
    expect(screen.queryByTestId('register-tier-0')).not.toBeInTheDocument()
    expect(screen.queryByTestId('register-tier-255')).not.toBeInTheDocument()
    expect(screen.getAllByTestId(/^register-tier-/)).toHaveLength(9)
  })

  it('connected registered tier 2: Active + Deregister; no Register on that row', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(getRegistration).mockResolvedValue(registeredAt(2))
    await renderTiers()

    expect(screen.getByTestId('tiers-your-status')).toHaveTextContent(/Tier 2/)
    expect(screen.getByTestId('tiers-deregister')).toBeInTheDocument()
    expect(within(screen.getByTestId('tier-card-2')).getByText('Active')).toBeInTheDocument()
    expect(screen.queryByTestId('register-tier-2')).not.toBeInTheDocument()
    expect(screen.getByTestId('register-tier-1')).toBeInTheDocument()
    expect(screen.getByTestId('register-tier-3')).toBeInTheDocument()
    expect(screen.getAllByTestId(/^register-tier-/)).toHaveLength(8)
  })

  it('Register sends that row’s tier_id only (wrong-row guard)', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    await renderTiers()

    await user.click(screen.getByTestId('register-tier-5'))
    await waitFor(() => {
      expect(register).toHaveBeenCalledTimes(1)
    })
    expect(register).toHaveBeenCalledWith(WALLET, 5)
    expect(register).not.toHaveBeenCalledWith(WALLET, 4)
    expect(register).not.toHaveBeenCalledWith(WALLET, 6)
  })

  it('pending Register disables every row button (no remount-enabled CTA)', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    let resolveRegister: (value: string) => void = () => undefined
    vi.mocked(register).mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveRegister = resolve
        })
    )
    await renderTiers()

    await user.click(screen.getByTestId('register-tier-3'))
    await waitFor(() => {
      expect(screen.getByTestId('register-tier-3')).toBeDisabled()
    })
    expect(screen.getByTestId('register-tier-1')).toBeDisabled()
    expect(screen.getByTestId('register-tier-9')).toBeDisabled()
    resolveRegister('tx')
  })

  it('governance_only fixtures never appear as self-register cards', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    await renderTiers()

    expect(screen.queryByTestId('tier-card-0')).not.toBeInTheDocument()
    expect(screen.queryByTestId('tier-card-255')).not.toBeInTheDocument()
    expect(screen.queryByText(/Governance only/i)).not.toBeInTheDocument()
    expect(screen.getAllByTestId(/^tier-card-/)).toHaveLength(9)
  })

  it('fee labels use factory default_fee_bps (I4) and tier 9 limit place is 0 (I13)', async () => {
    await renderTiers()

    const cluster1 = screen.getByTestId('tier-fee-cluster-1')
    expect(cluster1).toHaveTextContent('2.5%')
    expect(cluster1).toHaveTextContent('1.75%')

    const cluster9 = screen.getByTestId('tier-fee-cluster-9')
    expect(cluster9).toHaveTextContent('95%')
    expect(cluster9).toHaveTextContent('0.09%')

    const how = screen.getByTestId('tiers-how-it-works')
    expect(how).toHaveTextContent(/Limit place\*/)
    expect(how).toHaveTextContent('0%')
    expect(how).toHaveTextContent(/some pairs may have a different base fee/i)
    expect(how.querySelector('.font-mono')).toHaveTextContent('terra1cl8y')
    expect(how.innerHTML).not.toMatch(/dangerouslySetInnerHTML/)
  })

  it('missing VITE_FEE_DISCOUNT_ADDRESS shows not configured', () => {
    mockFeeDiscountAddr.current = ''
    renderWithProviders(<TiersPage />, { route: '/tiers' })
    expect(screen.getByTestId('tiers-not-configured')).toHaveTextContent('Fee discount contract not configured.')
    expect(getTiers).not.toHaveBeenCalled()
  })

  it('tiers query error shows RetryError and retry refetches', async () => {
    const user = userEvent.setup()
    vi.mocked(getTiers).mockRejectedValueOnce(new Error('LCD down')).mockResolvedValueOnce(FIXTURE_TIERS)
    renderWithProviders(<TiersPage />, { route: '/tiers' })

    expect(await screen.findByTestId('tiers-retry-error')).toBeInTheDocument()
    expect(screen.getByTestId('retry-error-button')).toBeInTheDocument()

    await user.click(screen.getByTestId('retry-error-button'))
    expect(await screen.findByTestId('tier-hold-1')).toHaveTextContent('Hold 1 CL8Y')
    expect(getTiers).toHaveBeenCalledTimes(2)
  })

  it('does not ship a ghost Register CTA or *-neo classes', async () => {
    await renderTiers()
    expect(screen.queryByRole('button', { name: /^Register$/i })).not.toBeInTheDocument()
    expect(document.body.innerHTML).not.toMatch(/-neo/)
  })
})
