import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import PoolPage from './PoolPage'
import { useWalletStore } from '@/hooks/useWallet'
import { FEE_DISCOUNT_REGISTRY_WARNING_TEXT } from '@/utils/feeDiscountRegistryWarning'
import { FEE_DISCOUNT_UNREGISTERED_CTA_TEXT } from '@/utils/feeDiscountUiCopy'
import type { IndexerPair } from '@/types'

// GitLab #476: Pool fee-discount CTA + registry outage banner require a configured contract.
vi.mock('@/utils/constants', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FEE_DISCOUNT_CONTRACT_ADDRESS: 'terra1feediscount000000000000000000000000001',
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
    getFeeDiscountHealth: vi.fn(),
  }
})

vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn().mockResolvedValue({ pairs: [] }),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: 'tokenA' } }, amount: '1000000' },
      { info: { token: { contract_addr: 'tokenB' } }, amount: '2000000' },
    ],
    total_share: '2000000',
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
    fee_bps: 180,
    treasury: '',
  }),
}))

vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({
    discount_bps: 2500,
    needs_deregister: false,
    registration_epoch: 1,
  }),
  getRegistration: vi.fn().mockResolvedValue({ registered: true, tier_id: 1, tier: null }),
}))

vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn().mockResolvedValue('terra1feediscount000000000000000000000000001'),
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

import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { TRADING_BLACKLIST_ALLOWED } from '@/test/tradingBlacklistMocks'
import * as indexerClient from '@/services/indexer/client'
import { getRegistration, getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getPairDiscountRegistry } from '@/services/terraclassic/pairDiscountRegistry'

const wallet = 'terra1wallet000000000000000000000000000001'

const mockPair: IndexerPair = {
  pair_address: 'terra1pair0000000000000000000000000000000',
  asset_0: { symbol: 'TKA', contract_addr: 'tokenA', denom: null, decimals: 6 },
  asset_1: { symbol: 'TKB', contract_addr: 'tokenB', denom: null, decimals: 6 },
  lp_token: 'lptoken1',
  fee_bps: 180,
  is_active: true,
}

describe('PoolPage fee-discount UX (GitLab #476)', () => {
  beforeEach(() => {
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED as never)
    vi.mocked(getAllPairsPaginated).mockResolvedValue({ pairs: [] })
    vi.mocked(indexerClient.getPairs).mockResolvedValue({
      total: 1,
      items: [mockPair],
      limit: 20,
      offset: 0,
    })
    vi.mocked(indexerClient.getTokens).mockResolvedValue([])
    vi.mocked(getRegistration).mockResolvedValue({ registered: true, tier_id: 1, tier: null })
    vi.mocked(getTraderDiscount).mockResolvedValue({
      discount_bps: 2500,
      needs_deregister: false,
      registration_epoch: 1,
    })
    vi.mocked(getPairDiscountRegistry).mockResolvedValue('terra1feediscount000000000000000000000000001')
    vi.mocked(indexerClient.getFeeDiscountHealth).mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })
  })

  async function openManage() {
    const user = userEvent.setup()
    const manage = await screen.findByTestId('pool-row-manage')
    await user.click(manage)
    return user
  }

  it('shows strikethrough discounted fee on the pool card when discount_bps > 0', async () => {
    renderWithProviders(<PoolPage />)
    await openManage()
    const badge = await screen.findByTestId('pool-fee-badge')
    await waitFor(() => {
      expect(badge).toHaveTextContent('1.35%')
      expect(badge.querySelector('.line-through')).toBeTruthy()
    })
    expect(badge).toHaveTextContent('1.80%')
  })

  it('shows full fee_bps with no strikethrough when the pair discount_registry is unset (#537)', async () => {
    vi.mocked(getPairDiscountRegistry).mockResolvedValue(null)

    renderWithProviders(<PoolPage />)
    await openManage()
    const badge = await screen.findByTestId('pool-fee-badge')
    expect(badge).toHaveTextContent('1.80%')
    expect(badge).not.toHaveTextContent('1.35%')
    expect(badge.querySelector('.line-through')).toBeNull()
    expect(screen.queryByTestId('pool-fee-discount-unregistered-cta')).not.toBeInTheDocument()
  })

  it('shows unregistered CTA and not-registered hint when wallet is unregistered', async () => {
    vi.mocked(getRegistration).mockResolvedValue({ registered: false, tier_id: null, tier: null })
    vi.mocked(getTraderDiscount).mockResolvedValue({
      discount_bps: 0,
      needs_deregister: false,
      registration_epoch: 0,
    })

    renderWithProviders(<PoolPage />)

    await openManage()
    const cta = await screen.findByTestId('pool-fee-discount-unregistered-cta')
    expect(cta).toHaveTextContent(FEE_DISCOUNT_UNREGISTERED_CTA_TEXT)
    expect(cta.querySelector('a')?.getAttribute('href')).toBe('/tiers')

    const badge = await screen.findByTestId('pool-fee-badge')
    expect(badge).toHaveTextContent('1.80%')
    expect(badge).toHaveTextContent(/not registered/i)
    expect(badge.querySelector('.line-through')).toBeNull()
    expect(screen.queryByTestId('pool-fee-discount-registry-warning')).not.toBeInTheDocument()
  })

  it('shows non-blocking registry outage warning for a registered trader when indexer health is down', async () => {
    vi.mocked(indexerClient.getFeeDiscountHealth).mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: false,
      consecutive_lcd_failures: 3,
    })

    renderWithProviders(<PoolPage />)

    const banner = await screen.findByTestId('pool-fee-discount-registry-warning')
    expect(banner).toHaveTextContent(FEE_DISCOUNT_REGISTRY_WARNING_TEXT)
    expect(screen.queryByTestId('pool-fee-discount-unregistered-cta')).not.toBeInTheDocument()
  })

  it('shows the warning when registration LCD query errors for a connected trader', async () => {
    vi.mocked(getRegistration).mockRejectedValue(new Error('LCD registration query failed'))

    renderWithProviders(<PoolPage />)

    expect(await screen.findByTestId('pool-fee-discount-registry-warning')).toHaveTextContent(
      FEE_DISCOUNT_REGISTRY_WARNING_TEXT
    )
  })

  it('does not show outage warning when a registered trader has a healthy registry', async () => {
    renderWithProviders(<PoolPage />)
    await openManage()
    await waitFor(() => expect(screen.getByTestId('pool-fee-badge')).toBeInTheDocument())
    expect(screen.queryByTestId('pool-fee-discount-registry-warning')).not.toBeInTheDocument()
  })

  it('does not show header eligibility essay; I14 CTA stays on Manage expand (GitLab #547)', async () => {
    renderWithProviders(<PoolPage />)
    await screen.findByTestId('pool-pairs-table')
    expect(screen.queryByTestId('pool-fee-discount-eligibility-note')).not.toBeInTheDocument()
  })

  it('shows full pair fee without strikethrough when the pair discount_registry is unset (#537)', async () => {
    vi.mocked(getPairDiscountRegistry).mockResolvedValue(null)

    renderWithProviders(<PoolPage />)
    await openManage()
    const badge = await screen.findByTestId('pool-fee-badge')
    expect(badge).toHaveTextContent('1.80%')
    expect(badge).not.toHaveTextContent('1.35%')
    expect(badge.querySelector('.line-through')).toBeNull()
    expect(badge).not.toHaveTextContent(/not registered/i)
  })

  it('does not advertise a VITE discount when the pair registry is a different contract (#537)', async () => {
    vi.mocked(getPairDiscountRegistry).mockResolvedValue('terra1otherdiscountregistry0000000000000000000001')

    renderWithProviders(<PoolPage />)
    await openManage()
    const badge = await screen.findByTestId('pool-fee-badge')
    expect(badge).toHaveTextContent('1.80%')
    expect(badge.querySelector('.line-through')).toBeNull()
  })
})
