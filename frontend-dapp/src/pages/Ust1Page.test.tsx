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

  it('loads deposit/withdraw tabs and oracle fee surface', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<Ust1Page />)
    expect(await screen.findByTestId('ust1-mode-tabs')).toBeInTheDocument()
    expect(screen.getByTestId('ust1-tab-deposit')).toBeInTheDocument()
    expect(screen.getByTestId('ust1-tab-withdraw')).toBeInTheDocument()
    expect(screen.getByText(/not an AMM swap/i)).toBeInTheDocument()
    expect(screen.getByTestId('ust1-oracle-status')).toHaveTextContent('Fresh')
    expect(screen.getByText('1.00%')).toBeInTheDocument()
  })

  it('disables CTA when window paused', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.getUst1EffectiveSwap).mockResolvedValue({ ...healthy, paused: true } as never)
    renderWithProviders(<Ust1Page />)
    await screen.findByTestId('ust1-submit')
    expect(screen.getByTestId('ust1-submit')).toBeDisabled()
    expect(screen.getByTestId('ust1-submit')).toHaveTextContent(/paused/i)
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

  it('quotes receive and submits deposit via window client', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(ust1Window.executeUst1Window).mockResolvedValue('txhash506')
    renderWithProviders(<Ust1Page />)
    await screen.findByTestId('ust1-pay-amount')
    await user.type(screen.getByTestId('ust1-pay-amount'), '1')
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
  })
})
