import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { WalletDropdownMenuItems } from '../WalletDropdownMenuItems'
import { copyToClipboard } from '@/utils/copyToClipboard'
import { getExplorerAddressUrl } from '@/utils/terraExplorer'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue({ ok: true }),
}))

vi.mock('@/utils/terraExplorer', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/terraExplorer')>()
  return {
    ...actual,
    getExplorerAddressUrl: vi.fn((addr: string) => `https://explorer.example/address/${addr}`),
  }
})

const ADDR = 'terra1connected000000000000000000000000000'

describe('WalletDropdownMenuItems (GitLab #185 / #671)', () => {
  const onClose = vi.fn()
  const onSwitchWallet = vi.fn()

  beforeEach(() => {
    vi.mocked(getExplorerAddressUrl).mockReset()
    vi.mocked(getExplorerAddressUrl).mockImplementation((addr: string) => `https://explorer.example/address/${addr}`)
    onClose.mockReset()
    onSwitchWallet.mockReset()
  })

  it('renders copy, explorer, and switch wallet menu items', () => {
    render(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    expect(screen.getByTestId('wallet-menu-copy-address')).toHaveTextContent('Copy address')
    expect(screen.getByRole('menuitem', { name: 'View on explorer' })).toHaveAttribute(
      'href',
      `https://explorer.example/address/${ADDR}`
    )
    expect(screen.getByRole('menuitem', { name: 'View on explorer' })).toHaveAttribute('target', '_blank')
    expect(screen.getByRole('menuitem', { name: 'View on explorer' })).toHaveAttribute(
      'rel',
      expect.stringContaining('noopener')
    )
    expect(screen.getByRole('menuitem', { name: 'Switch wallet' })).toBeInTheDocument()
  })

  it('uses wallet-menu-item on every labeled row (#671)', () => {
    render(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    expect(screen.getByTestId('wallet-menu-copy-address')).toHaveClass('wallet-menu-item')
    for (const name of ['View on explorer', 'Switch wallet']) {
      expect(screen.getByRole('menuitem', { name })).toHaveClass('wallet-menu-item')
    }
  })

  it('copies the connected address string only', async () => {
    const user = userEvent.setup()
    render(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    await user.click(screen.getByTestId('wallet-menu-copy-address'))
    expect(copyToClipboard).toHaveBeenCalledWith(ADDR)
  })

  it('omits explorer row when helper returns null', () => {
    vi.mocked(getExplorerAddressUrl).mockReturnValue(null)
    render(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    expect(screen.queryByTestId('wallet-menu-view-explorer')).not.toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'View on explorer' })).not.toBeInTheDocument()
  })

  it('omits explorer row when helper returns javascript: or data: href (#671)', () => {
    vi.mocked(getExplorerAddressUrl).mockReturnValue('javascript:alert(1)')
    const { rerender } = render(
      <WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />
    )
    expect(screen.queryByTestId('wallet-menu-view-explorer')).not.toBeInTheDocument()

    vi.mocked(getExplorerAddressUrl).mockReturnValue('data:text/html,pwned')
    rerender(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    expect(screen.queryByTestId('wallet-menu-view-explorer')).not.toBeInTheDocument()
  })

  it('calls onSwitchWallet when Switch wallet is clicked', async () => {
    const user = userEvent.setup()
    render(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    await user.click(screen.getByRole('menuitem', { name: 'Switch wallet' }))
    expect(onSwitchWallet).toHaveBeenCalledTimes(1)
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})
