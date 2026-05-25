import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { WalletDropdownMenuItems } from '../WalletDropdownMenuItems'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue({ ok: true }),
}))

vi.mock('@/utils/terraExplorer', () => ({
  getExplorerAddressUrl: vi.fn((addr: string) => `https://explorer.example/address/${addr}`),
}))

const ADDR = 'terra1connected000000000000000000000000000'

describe('WalletDropdownMenuItems (GitLab #185)', () => {
  const onClose = vi.fn()
  const onSwitchWallet = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders copy, explorer, and switch wallet menu items', () => {
    render(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    expect(screen.getByTestId('wallet-menu-copy-address')).toHaveTextContent('Copy address')
    expect(screen.getByRole('menuitem', { name: 'View on explorer' })).toHaveAttribute(
      'href',
      `https://explorer.example/address/${ADDR}`
    )
    expect(screen.getByRole('menuitem', { name: 'Switch wallet' })).toBeInTheDocument()
  })

  it('calls onSwitchWallet when Switch wallet is clicked', async () => {
    const user = userEvent.setup()
    render(<WalletDropdownMenuItems address={ADDR} onClose={onClose} onSwitchWallet={onSwitchWallet} />)
    await user.click(screen.getByRole('menuitem', { name: 'Switch wallet' }))
    expect(onSwitchWallet).toHaveBeenCalledTimes(1)
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})
