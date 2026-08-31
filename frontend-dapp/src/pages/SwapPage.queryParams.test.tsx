import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'

const { UST1, LISTED } = vi.hoisted(() => ({
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  LISTED: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
}))

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))
vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn(),
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
      { info: { native_token: { denom: 'uluna' } }, amount: '1000000' },
      { info: { token: { contract_addr: UST1 } }, amount: '1000000' },
    ],
    total_share: '1000000',
  }),
}))
vi.mock('@/services/terraclassic/assetCodeIdFreeze', () => ({
  probePairCodeIdFreeze: vi.fn().mockResolvedValue({ frozen: false, verdict: 'tradable' }),
}))
vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({ fee_bps: 30, treasury: '' }),
}))
vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({
    discount_bps: 0,
    needs_deregister: false,
    registration_epoch: null,
  }),
  getRegistration: vi.fn().mockResolvedValue({ registered: false, tier_id: null, tier: null }),
}))
vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn().mockResolvedValue(''),
}))
vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
  enrichSwapOperationsWithHopMinReturns: vi.fn(async (operations: unknown[]) => operations),
}))
vi.mock('@/services/terraclassic/router', () => ({
  findRoute: vi.fn(),
  getAllTokens: vi.fn(),
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
import { findRoute, getAllTokens } from '@/services/terraclassic/router'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import * as indexerClient from '@/services/indexer/client'

function seedFactoryTokens() {
  vi.mocked(getAllPairsPaginated).mockResolvedValue({
    pairs: [
      {
        contract_addr: 'terra1pair00000000000000000000000000000001',
        liquidity_token: 'terra1lp000000000000000000000000000000001',
        asset_infos: [{ native_token: { denom: 'uluna' } }, { token: { contract_addr: UST1 } }],
      },
    ],
  })
  vi.mocked(getAllTokens).mockReturnValue(['uluna', 'uusd', UST1, LISTED])
  vi.mocked(findRoute).mockReturnValue([
    {
      terra_swap: {
        offer_asset_info: { native_token: { denom: 'uluna' } },
        ask_asset_info: { token: { contract_addr: UST1 } },
      },
    },
  ] as never)
}

async function waitForSwapReady() {
  await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
  await waitFor(() => expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toBeEnabled())
}

describe('SwapPage query params (GitLab #711)', () => {
  beforeEach(() => {
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    seedFactoryTokens()
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
  })

  it('AC1: /?from=uluna&to=<listed cw20> selects LUNC and that token', async () => {
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC1: waits for factory pairs so wrap natives do not drop a listed CW20', async () => {
    const factoryPairs = [
      {
        contract_addr: 'terra1pair00000000000000000000000000000001',
        liquidity_token: 'terra1lp000000000000000000000000000000001',
        asset_infos: [{ native_token: { denom: 'uluna' } }, { token: { contract_addr: UST1 } }],
      },
    ]
    let resolvePairs: (value: { pairs: typeof factoryPairs }) => void = () => {}
    vi.mocked(getAllPairsPaginated).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePairs = resolve
        })
    )
    vi.mocked(getAllTokens).mockImplementation((pairs: { asset_infos?: unknown[] }[] = []) =>
      pairs.length === 0 ? ['uluna', 'uusd'] : ['uluna', 'uusd', UST1, LISTED]
    )
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    expect(screen.getByText(/loading pairs/i)).toBeInTheDocument()
    resolvePairs({ pairs: factoryPairs })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC2: Uniswap inputCurrency / outputCurrency', async () => {
    renderWithProviders(<SwapPage />, { route: `/?inputCurrency=uusd&outputCurrency=${UST1}` })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('USTC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC2 one-sided: outputCurrency only defaults pay', async () => {
    renderWithProviders(<SwapPage />, { route: `/?outputCurrency=${UST1}` })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC5: hostile values are ignored and not echoed in the trigger', async () => {
    renderWithProviders(<SwapPage />, { route: '/?from=javascript:alert(1)&to=%3Cscript%3E' })
    await waitForSwapReady()
    const pay = screen.getByRole('combobox', { name: 'Select token you pay' })
    const receive = screen.getByRole('combobox', { name: 'Select token you receive' })
    expect(pay).toHaveValue('LUNC')
    expect(receive).toHaveValue('USTC')
    expect((pay as HTMLInputElement).value).not.toMatch(/javascript/i)
    expect((receive as HTMLInputElement).value).not.toMatch(/script/i)
  })

  it('AC6: exactAmount prefills You Pay and does not auto-submit', async () => {
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}&exactAmount=1.5` })
    await waitForSwapReady()
    expect(screen.getByPlaceholderText('0.00')).toHaveValue('1.5')
    expect(screen.getByRole('button', { name: /connect wallet/i })).toBeEnabled()
    expect(screen.queryByRole('button', { name: /^swap$/i })).not.toBeInTheDocument()
  })

  it('AC7: user can change receive after apply; query is not re-forced', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    await waitForSwapReady()
    await user.click(screen.getByRole('combobox', { name: 'Select token you receive' }))
    await screen.findByRole('listbox', { name: 'Select token you receive' })
    await user.click(screen.getByRole('option', { name: /USTC/i }))
    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('USTC')
    })
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
  })

  it('AC7: flip pay/receive after apply', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    await waitForSwapReady()
    await user.click(screen.getByRole('button', { name: 'Swap pay and receive tokens' }))
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('UST1')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('LUNC')
  })
})
