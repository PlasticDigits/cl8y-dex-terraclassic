import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import WrapPage from './WrapPage'
import { useWalletStore } from '@/hooks/useWallet'
import * as wrapMapper from '@/services/terraclassic/wrapMapper'
import * as router from '@/services/terraclassic/router'

const { WALLET, LUNC_C, USTC_C, TREASURY, MAPPER, mockEnabled } = vi.hoisted(() => ({
  WALLET: 'terra1wallet00000000000000000000000000001',
  LUNC_C: 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg',
  USTC_C: 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch',
  TREASURY: 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2',
  MAPPER: 'terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2',
  mockEnabled: vi.fn(() => true),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    WRAP_MAPPER_CONTRACT_ADDRESS: MAPPER,
    TREASURY_CONTRACT_ADDRESS: TREASURY,
    LUNC_C_TOKEN_ADDRESS: LUNC_C,
    USTC_C_TOKEN_ADDRESS: USTC_C,
    isNativeWrapEnabled: () => mockEnabled(),
  }
})

vi.mock('@/services/terraclassic/wrapMapper', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/wrapMapper')>()
  return {
    ...actual,
    queryWrapMapperConfig: vi.fn(),
    queryPausedState: vi.fn(),
    checkRateLimitExceeded: vi.fn(),
    queryRateLimit: vi.fn(),
  }
})

vi.mock('@/services/terraclassic/router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/router')>()
  return {
    ...actual,
    simulateNativeSwap: vi.fn(),
    executeNativeSwap: vi.fn(),
  }
})

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

const healthyConfig = {
  governance: 'terra1gov',
  treasury: TREASURY,
  paused: false,
  fee_bps: 200,
}

describe('WrapPage (#502 / #507)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockEnabled.mockReturnValue(true)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.mocked(wrapMapper.queryWrapMapperConfig).mockResolvedValue(healthyConfig)
    vi.mocked(wrapMapper.queryPausedState).mockResolvedValue(false)
    vi.mocked(wrapMapper.checkRateLimitExceeded).mockResolvedValue(false)
    vi.mocked(wrapMapper.queryRateLimit).mockResolvedValue({
      config: { max_amount_per_window: '110000000000000', window_seconds: 86400 },
      current_window_start: null,
      amount_used: '0',
    })
    vi.mocked(router.simulateNativeSwap).mockResolvedValue({ amount: '980000', isDirectWrapUnwrap: true })
  })

  it('shows unavailable when wrap env is missing', () => {
    mockEnabled.mockReturnValue(false)
    renderWithProviders(<WrapPage />)
    expect(screen.getByTestId('wrap-unavailable')).toBeInTheDocument()
  })

  it('loads wrap/unwrap tabs and fee surface', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<WrapPage />)
    expect(await screen.findByTestId('wrap-mode-tabs')).toBeInTheDocument()
    expect(screen.getByTestId('wrap-tab-wrap')).toBeInTheDocument()
    expect(screen.getByTestId('wrap-tab-unwrap')).toBeInTheDocument()
    expect(screen.getByText(/not an AMM swap/i)).toBeInTheDocument()
    expect(screen.getByTestId('wrap-pay-symbol')).toHaveTextContent('Pay (LUNC)')
    expect(screen.getByTestId('wrap-receive-symbol')).toHaveTextContent('Receive (cLUNC)')
    expect(await screen.findByTestId('wrap-fee-note')).toHaveTextContent(/2/)
    expect(await screen.findByTestId('wrap-page-rate-limit-available')).toBeInTheDocument()
  })

  it('switches asset to USTC / cUSTC and shows logos on asset toggles', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<WrapPage />)
    const luncBtn = await screen.findByTestId('wrap-asset-lunc')
    expect(luncBtn.querySelectorAll('img').length).toBeGreaterThanOrEqual(1)
    await user.click(screen.getByTestId('wrap-asset-ustc'))
    expect(screen.getByTestId('wrap-pay-symbol')).toHaveTextContent('Pay (USTC)')
    expect(screen.getByTestId('wrap-receive-symbol')).toHaveTextContent('Receive (cUSTC)')
    expect(screen.getByTestId('wrap-asset-ustc').querySelectorAll('img').length).toBeGreaterThanOrEqual(1)
  })

  it('disables CTA when wrap-mapper paused', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(wrapMapper.queryWrapMapperConfig).mockResolvedValue({ ...healthyConfig, paused: true })
    vi.mocked(wrapMapper.queryPausedState).mockResolvedValue(true)
    renderWithProviders(<WrapPage />)
    await screen.findByTestId('wrap-submit')
    expect(screen.getByTestId('wrap-submit')).toBeDisabled()
    expect(screen.getByTestId('wrap-submit')).toHaveTextContent(/paused/i)
  })

  it('quotes receive amount and enables wrap submit', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<WrapPage />)
    await screen.findByTestId('wrap-pay-amount')
    await user.type(screen.getByTestId('wrap-pay-amount'), '1')
    await waitFor(() => {
      expect(screen.getByTestId('wrap-receive-amount')).toHaveTextContent('0.98')
    })
    expect(screen.getByTestId('wrap-submit')).not.toBeDisabled()
    expect(screen.getByTestId('wrap-submit')).toHaveTextContent(/^Wrap$/)
  })
})
