import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import WalletButton from '../WalletButton'

vi.mock('@/hooks/useWallet', () => ({
  useWalletStore: vi.fn(),
}))

vi.mock('../WalletLuncBalance', () => ({
  WalletLuncBalance: ({ address }: { address: string }) => (
    <span data-testid="wallet-lunc-balance-mock">{address.slice(0, 6)} LUNC</span>
  ),
}))

vi.mock('./WalletModal', () => ({
  default: () => null,
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

import { useWalletStore } from '@/hooks/useWallet'

const mockUseWalletStore = vi.mocked(useWalletStore)

const ADDR = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

describe('WalletButton connected LUNC (GitLab #140)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUseWalletStore.mockReturnValue({
      address: ADDR,
      isConnecting: false,
      disconnect: vi.fn(),
      walletModalOpen: false,
      setWalletModalOpen: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
  })

  it('renders LUNC balance in the connected chip without opening the menu', () => {
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    const balances = screen.getAllByTestId('wallet-lunc-balance-mock')
    expect(balances.length).toBeGreaterThanOrEqual(1)
    expect(balances[0]).toHaveTextContent('terra1 LUNC')
  })

  it('shows LUNC balance and full address in the dropdown header', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    await user.click(screen.getByRole('button', { expanded: false }))
    expect(screen.getByText(ADDR)).toBeInTheDocument()
    expect(screen.getAllByTestId('wallet-lunc-balance-mock').length).toBeGreaterThanOrEqual(2)
  })

  it('shows network shortLabel on the connected trigger (GitLab #186)', () => {
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    expect(screen.getByTestId('wallet-network-short-label')).toHaveTextContent('Local')
    expect(screen.getByRole('button', { name: /Connected wallet on Local/i })).toBeInTheDocument()
  })
})

describe('WalletButton menu dismiss (GitLab #187)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUseWalletStore.mockReturnValue({
      address: ADDR,
      isConnecting: false,
      disconnect: vi.fn(),
      walletModalOpen: false,
      setWalletModalOpen: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
  })

  const openWalletMenu = async (user: ReturnType<typeof userEvent.setup>) => {
    await user.click(screen.getByRole('button', { expanded: false }))
    expect(screen.getByRole('menu')).toBeInTheDocument()
  }

  it('renders a semantic dismiss control when the menu is open', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    await openWalletMenu(user)
    expect(screen.getByRole('button', { name: 'Close wallet menu' })).toBeInTheDocument()
  })

  it('closes the menu when the dismiss control is clicked', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    await openWalletMenu(user)
    await user.click(screen.getByRole('button', { name: 'Close wallet menu' }))
    expect(screen.queryByRole('menu')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument()
  })

  it('closes the menu on Escape without leaving the chip expanded', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    await openWalletMenu(user)
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('menu')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument()
  })

  it('moves focus into the menu on open and returns to trigger on Escape (GitLab #214)', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    const trigger = screen.getByRole('button', { name: /Connected wallet on Local/i })
    await user.click(trigger)
    await waitFor(() => expect(screen.getByTestId('wallet-menu-copy-address')).toHaveFocus())
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('menu')).not.toBeInTheDocument()
    await waitFor(() => expect(trigger).toHaveFocus())
  })
})

describe('WalletButton dropdown affordances (GitLab #185)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUseWalletStore.mockReturnValue({
      address: ADDR,
      isConnecting: false,
      disconnect: vi.fn().mockResolvedValue(undefined),
      walletModalOpen: false,
      setWalletModalOpen: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
  })

  it('shows copy, explorer, and switch wallet rows when the menu is open', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    await user.click(screen.getByRole('button', { expanded: false }))
    expect(screen.getByTestId('wallet-menu-copy-address')).toHaveTextContent('Copy address')
    expect(screen.getByRole('menuitem', { name: 'View on explorer' })).toBeInTheDocument()
    expect(screen.getByRole('menuitem', { name: 'Switch wallet' })).toBeInTheDocument()
  })

  it('opens connect modal after Switch wallet', async () => {
    const disconnect = vi.fn().mockResolvedValue(undefined)
    const setWalletModalOpen = vi.fn()
    mockUseWalletStore.mockReturnValue({
      address: ADDR,
      isConnecting: false,
      disconnect,
      walletModalOpen: false,
      setWalletModalOpen,
    } as ReturnType<typeof useWalletStore>)

    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    await user.click(screen.getByRole('button', { expanded: false }))
    await user.click(screen.getByRole('menuitem', { name: 'Switch wallet' }))
    expect(disconnect).toHaveBeenCalledTimes(1)
    await vi.waitFor(() => expect(setWalletModalOpen).toHaveBeenCalledWith(true))
  })
})

describe('WalletButton connecting cancel (GitLab #554)', () => {
  it('shows Cancel instead of a disabled spinner', async () => {
    const cancelConnection = vi.fn()
    mockUseWalletStore.mockReturnValue({
      address: null,
      isConnecting: true,
      disconnect: vi.fn(),
      walletModalOpen: false,
      setWalletModalOpen: vi.fn(),
      closeWalletModal: vi.fn(),
      cancelConnection,
    } as ReturnType<typeof useWalletStore>)

    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <WalletButton />
      </MemoryRouter>
    )
    const btn = screen.getByRole('button', { name: /cancel connecting/i })
    expect(btn).toHaveTextContent('Cancel')
    expect(btn).not.toBeDisabled()
    await user.click(btn)
    expect(cancelConnection).toHaveBeenCalled()
  })
})
