import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
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

const ADDR = 'terra1connected000000000000000000000000000'

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
})
