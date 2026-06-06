import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))
vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn().mockResolvedValue({ pairs: [] }),
}))

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue(null),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  simulateSwap: vi.fn().mockResolvedValue({
    return_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  swap: vi.fn().mockResolvedValue('txhash123'),
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: 'tokenA' } }, amount: '1000000' },
      { info: { token: { contract_addr: 'tokenB' } }, amount: '1000000' },
    ],
    total_share: '1000000',
  }),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({
    fee_bps: 30,
    treasury: '',
  }),
}))

vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({ discount_bps: 0 }),
  getRegistration: vi.fn().mockResolvedValue({ registered: false }),
}))

vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
}))

vi.mock('@/services/terraclassic/router', () => ({
  findRoute: vi.fn().mockReturnValue(null),
  getAllTokens: vi.fn().mockReturnValue([]),
  simulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '1000000' }),
  executeMultiHopSwap: vi.fn().mockResolvedValue('txhash123'),
  isDirectWrapUnwrap: vi.fn().mockReturnValue(null),
  findRouteWithNativeSupport: vi.fn().mockReturnValue(null),
  simulateNativeSwap: vi.fn().mockResolvedValue({ amount: '1' }),
  executeNativeSwap: vi.fn().mockResolvedValue('tx'),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { findRoute, getAllTokens, simulateMultiHopSwap } from '@/services/terraclassic/router'
import { simulateSwap } from '@/services/terraclassic/pair'
import * as indexerClient from '@/services/indexer/client'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { getTokenBalance } from '@/services/terraclassic/queries'

describe('SwapPage', () => {
  beforeEach(() => {
    vi.mocked(getAllPairsPaginated).mockResolvedValue({ pairs: [] })
    vi.mocked(findRoute).mockReturnValue(null)
    vi.mocked(getAllTokens).mockReturnValue([])
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
  })

  it('renders without crashing', async () => {
    renderWithProviders(<SwapPage />)
    expect(await screen.findByRole('heading', { name: /^swap$/i })).toBeInTheDocument()
  })

  it('shows loading pairs state while factory pairs fetch is pending', async () => {
    let resolvePairs!: (value: { pairs: [] }) => void
    vi.mocked(getAllPairsPaginated).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePairs = resolve
        })
    )
    renderWithProviders(<SwapPage />)
    expect(screen.getByText(/loading pairs/i)).toBeInTheDocument()
    resolvePairs({ pairs: [] })
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
  })

  it('shows hybrid book warning with doc link before swap when book leg > 0', async () => {
    const user = userEvent.setup()
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pair00000000000000000000000000000001',
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraB])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
    ] as never)
    vi.spyOn(indexerClient, 'getRouteSolve').mockReset()
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
    vi.spyOn(indexerClient, 'postRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraB,
      hops: [],
      router_operations: [],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '5000',
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    await user.click(screen.getByRole('checkbox', { name: /Route part of input through the limit book/i }))
    await user.type(screen.getByPlaceholderText('0.0'), '0.01')
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/integrator semantics/i)
    const docLink = within(alert).getByRole('link', { name: /docs\/limit-orders\.md/i })
    expect(docLink.getAttribute('href')).toContain('docs/limit-orders.md')

    const execution = await screen.findByTestId('swap-execution-summary')
    expect(execution).toHaveTextContent(/Execution:\s*Indexer hybrid/i)
    expect(execution).toHaveTextContent(/Hybrid \(pool \+ limit book\)/i)
  })

  it('shows market-data outage banner when sim fails with indexer transport error (GitLab #241)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pair00000000000000000000000000000001',
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraB])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
    ] as never)
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(simulateSwap).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    const banner = await screen.findByTestId('swap-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
    expect(banner).not.toHaveTextContent(/VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(screen.getByTestId('swap-quote-unavailable')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Quote unavailable' })).toBeDisabled()
  })

  it('shows outage banner with pool fallback quote when indexer fails but LCD sim succeeds (GitLab #241)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pair00000000000000000000000000000001',
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraB])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
    ] as never)
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(simulateSwap).mockResolvedValue({
      return_amount: '1000000',
      spread_amount: '100',
      commission_amount: '3000',
    })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    expect(await screen.findByTestId('swap-market-data-outage-banner')).toBeInTheDocument()
    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
  })

  it('does not show outage banner when route/solve returns 400 but LCD fallback succeeds (GitLab #326)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pair00000000000000000000000000000001',
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraB])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
    ] as never)
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 400 Bad Request'))
    vi.mocked(simulateSwap).mockResolvedValue({
      return_amount: '1000000',
      spread_amount: '100',
      commission_amount: '3000',
    })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
    expect(screen.queryByTestId('swap-market-data-outage-banner')).not.toBeInTheDocument()
  })

  it('shows client BFS fallback label when multihop submit uses client graph without indexer ops (#329)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1aa0000000000000000000000000000000001'
    const terraB = 'terra1bb0000000000000000000000000000000001'
    const terraC = 'terra1cc0000000000000000000000000000000001'
    const multihopRoute = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraB } },
          ask_asset_info: { token: { contract_addr: terraC } },
        },
      },
    ] as never
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pairab000000000000000000000000000001',
          liquidity_token: 'terra1lpab000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
        {
          contract_addr: 'terra1pairbc000000000000000000000000000001',
          liquidity_token: 'terra1lpbc000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraB } }, { token: { contract_addr: terraC } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraC, terraB])
    vi.mocked(findRoute).mockReturnValue(multihopRoute)
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '1000000' })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    expect(await screen.findByTestId('swap-route-summary')).toBeInTheDocument()
    expect(await screen.findByTestId('swap-route-source-client-fallback')).toHaveTextContent(/client graph/i)
  })

  it('does not show client BFS fallback label when indexer multihop ops are used (#329)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1aa0000000000000000000000000000000001'
    const terraB = 'terra1bb0000000000000000000000000000000001'
    const terraC = 'terra1cc0000000000000000000000000000000001'
    const multihopRoute = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraB } },
          ask_asset_info: { token: { contract_addr: terraC } },
        },
      },
    ] as never
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pairab000000000000000000000000000001',
          liquidity_token: 'terra1lpab000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
        {
          contract_addr: 'terra1pairbc000000000000000000000000000001',
          liquidity_token: 'terra1lpbc000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraB } }, { token: { contract_addr: terraC } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraC, terraB])
    vi.mocked(findRoute).mockReturnValue(multihopRoute)
    vi.spyOn(indexerClient, 'getRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraC,
      hops: [
        { offer_token: terraA, ask_token: terraB },
        { offer_token: terraB, ask_token: terraC },
      ],
      router_operations: multihopRoute,
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '1000000',
      intermediate_tokens: [terraA, terraB, terraC],
    })
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '1000000' })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    await screen.findByTestId('swap-route-summary')
    expect(screen.queryByTestId('swap-route-source-client-fallback')).not.toBeInTheDocument()
  })

  it('rejects invalid characters in book leg amount without surfacing BigInt errors', async () => {
    const user = userEvent.setup()
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pair00000000000000000000000000000001',
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraB])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
    ] as never)

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    await user.click(screen.getByRole('checkbox', { name: /Route part of input through the limit book/i }))
    const bookInput = screen.getByPlaceholderText('0.0')
    await user.type(bookInput, '1^2')
    expect(bookInput).toHaveValue('12')
    expect(screen.queryByText(/Cannot convert/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/Something went wrong/i)).not.toBeInTheDocument()
  })

  it('blocks swap above 30% route slippage unless Expert Mode is enabled (GitLab #293)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pair00000000000000000000000000000001',
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraB])
    vi.mocked(findRoute).mockReturnValue(null)
    vi.spyOn(indexerClient, 'getRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraB,
      hops: [{ pair: 'terra1pair', offer_token: terraA, ask_token: terraB }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: terraA } },
            ask_asset_info: { token: { contract_addr: terraB } },
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '36000000000',
      slippage_percent: '99.97',
      spot_amount_out: '960000',
    })
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '36000000000' })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    // Wallet-aligned slippage (return vs spot) rounds to 100% for this arb-sized gap.
    expect(await screen.findByTestId('swap-expected-slippage')).toHaveTextContent('100.00%')
    expect(screen.getByTestId('swap-slippage-blocked')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Slippage is too high' })).toBeDisabled()
    expect(screen.getByTestId('swap-extreme-slippage-warning')).toBeInTheDocument()

    await user.click(screen.getByTestId('swap-enable-expert-mode'))
    const modal = await screen.findByRole('dialog')
    await user.click(within(modal).getByRole('button', { name: 'Enable Expert Mode' }))

    await waitFor(() => {
      expect(screen.queryByTestId('swap-slippage-blocked')).not.toBeInTheDocument()
      expect(screen.queryByRole('button', { name: 'Slippage is too high' })).not.toBeInTheDocument()
    })
  })
})
