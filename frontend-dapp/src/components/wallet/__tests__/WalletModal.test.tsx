import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import { WalletName } from '@goblinhunt/cosmes/wallet'
import WalletModal from '../WalletModal'

vi.mock('@/hooks/useWallet', () => ({
  useWalletStore: vi.fn(),
}))

vi.mock('@/hooks/useWalletExtensionInstallSnapshot', () => ({
  useWalletExtensionInstallSnapshot: vi.fn(),
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return { ...actual, DEV_MODE: false }
})

import { useWalletStore } from '@/hooks/useWallet'
import { useWalletExtensionInstallSnapshot } from '@/hooks/useWalletExtensionInstallSnapshot'

const mockUseWalletStore = vi.mocked(useWalletStore)
const mockSnapshot = vi.mocked(useWalletExtensionInstallSnapshot)

function extensionSnapshot(partial: Partial<Record<WalletName, boolean>>): Map<WalletName, boolean> {
  const m = new Map<WalletName, boolean>()
  for (const id of [WalletName.STATION, WalletName.KEPLR, WalletName.COSMOSTATION]) {
    m.set(id, partial[id] ?? false)
  }
  return m
}

describe('WalletModal (GitLab #160 connect modal badges)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUseWalletStore.mockReturnValue({
      connect: vi.fn().mockResolvedValue(undefined),
      connectDev: vi.fn(),
      isConnecting: false,
      error: null,
    } as ReturnType<typeof useWalletStore>)
  })

  it('does not render a Not installed pill when an extension is missing (Install link remains)', () => {
    mockSnapshot.mockReturnValue(extensionSnapshot({}))
    render(<WalletModal onClose={() => {}} />)
    expect(screen.queryByText('Not installed')).not.toBeInTheDocument()
    const installs = screen.getAllByRole('link', { name: /^install$/i })
    expect(installs.length).toBeGreaterThanOrEqual(1)
  })

  it('shows Ready on an extension row only when that wallet is detected', () => {
    mockSnapshot.mockReturnValue(extensionSnapshot({ [WalletName.COSMOSTATION]: true }))
    render(<WalletModal onClose={() => {}} />)
    const cosmo = screen.getByRole('button', { name: /Cosmostation, extension detected/i })
    expect(within(cosmo).getByText('Ready')).toBeInTheDocument()
    const keplr = screen.getByRole('button', { name: /Keplr, extension not detected/i })
    expect(within(keplr).queryByText('Ready')).not.toBeInTheDocument()
  })
})
