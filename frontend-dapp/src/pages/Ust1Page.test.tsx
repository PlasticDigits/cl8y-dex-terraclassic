import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import Ust1Page from './Ust1Page'
import { useWalletStore } from '@/hooks/useWallet'
import * as ust1Window from '@/services/terraclassic/ust1Window'
import { UST1_RATE_SCALE } from '@/utils/ust1WindowMath'

const { WINDOW, WALLET, UST1, VFDUSD, mockEnabled } = vi.hoisted(() => ({
  WINDOW: 'terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2',
  WALLET: 'terra1wallet00000000000000000000000000001',
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  VFDUSD: 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3',
  mockEnabled: vi.fn(() => true),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    UST1_WINDOW_CONTRACT_ADDRESS: WINDOW,
    UST1_TOKEN_ADDRESS: UST1,
    VFDUSD_TOKEN_ADDRESS: VFDUSD,
    isUst1WindowEnabled: () => mockEnabled(),
  }
})

vi.mock('@/services/terraclassic/ust1Window', () => ({
  getUst1EffectiveSwap: vi.fn(),
  executeUst1Window: vi.fn(),
  payTokenForDirection: (d: 'deposit' | 'withdraw') => (d === 'deposit' ? VFDUSD : UST1),
  paySymbolForDirection: (d: 'deposit' | 'withdraw') => (d === 'deposit' ? 'vFDUSD' : 'UST1'),
  receiveSymbolForDirection: (d: 'deposit' | 'withdraw') => (d === 'deposit' ? 'UST1' : 'vFDUSD'),
}))

vi.mock('@/hooks/useTokenBalance', () => ({
  useTokenBalance: () => ({
    data: '100000000',
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
}))

vi.mock('@/hooks/useDebouncedValue', () => ({
  useDebouncedValue: <T,>(v: T) => v,
}))

const healthy = {
  fee_bps: 100,
  per_tx_ust1_limit: '1000000000',
  rolling_24h_ust1_limit: '10000000000',
  paused: false,
  rolling_window_start_sec: Math.floor(Date.now() / 1000) - 100,
  rolling_volume_ust1: '0',
  max_oracle_age_sec: 21_600,
  oracle: {
    rate: UST1_RATE_SCALE.toString(),
    last_update_sec: Math.floor(Date.now() / 1000) - 30,
    paused: false,
  },
}

async function typePayAmount(user: ReturnType<typeof userEvent.setup>, amount: string) {
  await screen.findByTestId('ust1-pay-amount')
  await user.clear(screen.getByTestId('ust1-pay-amount'))
  await user.type(screen.getByTestId('ust1-pay-amount'), amount)
}

describe('Ust1Page (#506)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockEnabled.mockReturnValue(true)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.mocked(ust1Window.getUst1EffectiveSwap).mockResolvedValue(healthy as never)
  })

  it('shows unavailable when window env is missing', () => {
    mockEnabled.mockReturnValue(false)
    renderWithProviders(<Ust1Page />)
    expect(screen.getByTestId('ust1-unavailable')).toBeInTheDocument()
  })

  it('loads deposit/withdraw tabs and oracle fee surface with token logos', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<Ust1Page />)
    expect(await screen.findByTestId('ust1-mode-tabs')).toBeInTheDocument()
    expect(screen.getByTestId('ust1-tab-deposit')).toBeInTheDocument()
    expect(screen.getByTestId('ust1-tab-withdraw')).toBeInTheDocument()
    expect(screen.getByText(/not an AMM swap/i)).toBeInTheDocument()
    expect(screen.getByTestId('ust1-oracle-status')).toHaveTextContent('Fresh')
    expect(screen.getByText('1.00%')).toBeInTheDocument()
    expect(screen.getByTestId('ust1-pay-symbol')).toHaveTextContent('Pay (vFDUSD)')
    expect(screen.getByTestId('ust1-receive-symbol')).toHaveTextContent('Receive (UST1)')
    expect(document.querySelectorAll('img[src*="tokenlist/images/"]').length).toBeGreaterThanOrEqual(1)
  })

  it('disables CTA when window paused', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.getUst1EffectiveSwap).mockResolvedValue({ ...healthy, paused: true } as never)
    renderWithProviders(<Ust1Page />)
    await screen.findByTestId('ust1-submit')
    expect(screen.getByTestId('ust1-submit')).toBeDisabled()
    expect(screen.getByTestId('ust1-submit')).toHaveTextContent(/paused/i)
  })

  it('disables CTA when oracle paused', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.getUst1EffectiveSwap).mockResolvedValue({
      ...healthy,
      oracle: { ...healthy.oracle, paused: true },
    } as never)
    renderWithProviders(<Ust1Page />)
    await screen.findByTestId('ust1-submit')
    expect(screen.getByTestId('ust1-submit')).toBeDisabled()
    expect(screen.getByTestId('ust1-submit')).toHaveTextContent(/oracle paused/i)
  })

  it('disables CTA when oracle stale', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.getUst1EffectiveSwap).mockResolvedValue({
      ...healthy,
      oracle: { ...healthy.oracle, last_update_sec: 1 },
    } as never)
    renderWithProviders(<Ust1Page />)
    await screen.findByTestId('ust1-submit')
    expect(screen.getByTestId('ust1-submit')).toBeDisabled()
    expect(screen.getByTestId('ust1-submit')).toHaveTextContent(/stale/i)
  })

  it('disables CTA and shows reason when over per-tx limit', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.getUst1EffectiveSwap).mockResolvedValue({
      ...healthy,
      per_tx_ust1_limit: '1000000', // 1 UST1
    } as never)
    renderWithProviders(<Ust1Page />)
    await typePayAmount(user, '10')
    await waitFor(() => {
      expect(screen.getByTestId('ust1-submit')).toBeDisabled()
      expect(screen.getByTestId('ust1-submit')).toHaveTextContent(/per-tx/i)
      expect(screen.getByTestId('ust1-block-reason')).toHaveTextContent(/per-transaction/i)
    })
  })

  it('disables CTA and shows reason when over rolling 24h limit', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.getUst1EffectiveSwap).mockResolvedValue({
      ...healthy,
      rolling_24h_ust1_limit: '5000000',
      rolling_volume_ust1: '4500000',
    } as never)
    renderWithProviders(<Ust1Page />)
    await typePayAmount(user, '2')
    await waitFor(() => {
      expect(screen.getByTestId('ust1-submit')).toBeDisabled()
      expect(screen.getByTestId('ust1-submit')).toHaveTextContent(/24h/i)
      expect(screen.getByTestId('ust1-block-reason')).toHaveTextContent(/24h/i)
    })
  })

  it('shows withdraw min-out slippage disclosure on Withdraw tab', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<Ust1Page />)
    await screen.findByTestId('ust1-tab-withdraw')
    await user.click(screen.getByTestId('ust1-tab-withdraw'))
    expect(await screen.findByTestId('ust1-withdraw-slippage-note')).toHaveTextContent(/1% minimum output/i)
    expect(screen.queryByTestId('ust1-withdraw-slippage-note')).toBeInTheDocument()
  })

  it('prefills deposit amount from a legal query string (#678)', async () => {
    renderWithProviders(<Ust1Page />, { route: '/ust1?direction=deposit&amount=10' })
    await waitFor(() => {
      expect((screen.getByTestId('ust1-pay-amount') as HTMLInputElement).value).toBe('10')
    })
    expect(screen.getByTestId('ust1-tab-deposit')).toHaveAttribute('aria-selected', 'true')
  })

  it('clamps huge prefill amounts to remaining window capacity (#678 A10)', async () => {
    renderWithProviders(<Ust1Page />, { route: '/ust1?direction=deposit&amount=999999999999999' })
    await waitFor(() => {
      const v = (screen.getByTestId('ust1-pay-amount') as HTMLInputElement).value
      expect(v).not.toBe('')
      expect(Number(v)).toBeLessThanOrEqual(1000 * 1.02)
    })
  })

  it('ignores hostile query amounts (#678 A10)', async () => {
    renderWithProviders(<Ust1Page />, { route: '/ust1?amount=abc&next=https://evil.example' })
    await screen.findByTestId('ust1-pay-amount')
    expect((screen.getByTestId('ust1-pay-amount') as HTMLInputElement).value).toBe('')
  })

  it('quotes receive and submits deposit via window client', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.executeUst1Window).mockResolvedValue('txhash506')
    renderWithProviders(<Ust1Page />)
    await typePayAmount(user, '1')
    await waitFor(() => {
      expect(screen.getByTestId('ust1-receive-amount').textContent).not.toBe('—')
    })
    const submit = screen.getByTestId('ust1-submit')
    expect(submit).not.toBeDisabled()
    await user.click(submit)
    await waitFor(() => {
      expect(ust1Window.executeUst1Window).toHaveBeenCalled()
    })
    expect(await screen.findByTestId('ust1-success')).toBeInTheDocument()
    expect(screen.getByText(/Submitted/i)).toBeInTheDocument()
  })
})
