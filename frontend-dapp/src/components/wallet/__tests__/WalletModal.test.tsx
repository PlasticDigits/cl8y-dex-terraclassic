import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import { WalletName } from '@goblinhunt/cosmes/wallet'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import WalletModal from '../WalletModal'
import {
  PRODUCTION_WALLET_NAMES,
  productionWalletIconSrcs,
  SIMULATED_WALLET_ICON_SRC,
  walletIconSrc,
} from '../walletIconSrc'

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

  it('does not list Leap (GitLab #159 vendor sunset)', () => {
    mockSnapshot.mockReturnValue(extensionSnapshot({}))
    render(<WalletModal onClose={() => {}} />)
    expect(screen.queryByText(/^leap$/i)).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /leap/i })).not.toBeInTheDocument()
  })
})

describe('WalletModal circular logos (GitLab #490)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockUseWalletStore.mockReturnValue({
      connect: vi.fn().mockResolvedValue(undefined),
      connectDev: vi.fn(),
      isConnecting: false,
      error: null,
    } as ReturnType<typeof useWalletStore>)
    mockSnapshot.mockReturnValue(extensionSnapshot({}))
  })

  it('maps every production wallet to a local /wallets/* asset', () => {
    const srcs = productionWalletIconSrcs()
    expect(srcs).toHaveLength(PRODUCTION_WALLET_NAMES.length)
    for (const src of srcs) {
      expect(src.startsWith('/wallets/')).toBe(true)
      expect(src.includes('http')).toBe(false)
    }
    expect(walletIconSrc(WalletName.LEAP)).toBeUndefined()
  })

  it('ships static files under public/wallets for each production icon', () => {
    const publicRoot = join(process.cwd(), 'public')
    for (const name of PRODUCTION_WALLET_NAMES) {
      const src = walletIconSrc(name)
      expect(src).toBeTruthy()
      const filePath = join(publicRoot, src!.replace(/^\//, ''))
      expect(existsSync(filePath), `missing asset for ${name}: ${filePath}`).toBe(true)
    }
    expect(existsSync(join(publicRoot, SIMULATED_WALLET_ICON_SRC.replace(/^\//, '')))).toBe(true)
  })

  it('renders a circular icon on every production wallet row', () => {
    render(<WalletModal onClose={() => {}} />)
    for (const name of PRODUCTION_WALLET_NAMES) {
      const icon = screen.getByTestId(`wallet-option-icon-${name}`)
      expect(icon).toBeInTheDocument()
      expect(icon).toHaveAttribute('aria-hidden', 'true')
      const img = icon.querySelector('img')
      expect(img).not.toBeNull()
      expect(img).toHaveAttribute('src', walletIconSrc(name))
      expect(img).toHaveAttribute('alt', '')
    }
  })

  it('keeps Install CTA and icon when an extension is missing', () => {
    mockSnapshot.mockReturnValue(extensionSnapshot({}))
    render(<WalletModal onClose={() => {}} />)
    const keplr = screen.getByRole('button', { name: /Keplr, extension not detected/i })
    expect(within(keplr).getByTestId(`wallet-option-icon-${WalletName.KEPLR}`)).toBeInTheDocument()
    expect(screen.getAllByRole('link', { name: /^install$/i }).length).toBeGreaterThanOrEqual(1)
  })

  it('keeps Ready badge and icon when an extension is installed', () => {
    mockSnapshot.mockReturnValue(extensionSnapshot({ [WalletName.KEPLR]: true }))
    render(<WalletModal onClose={() => {}} />)
    const keplr = screen.getByRole('button', { name: /Keplr, extension detected/i })
    expect(within(keplr).getByText('Ready')).toBeInTheDocument()
    expect(within(keplr).getByTestId(`wallet-option-icon-${WalletName.KEPLR}`)).toBeInTheDocument()
  })

  it('shows icons on WalletConnect rows without Install', () => {
    render(<WalletModal onClose={() => {}} />)
    const lunc = screen.getByRole('button', { name: /^LuncDash$/i })
    expect(within(lunc).getByTestId(`wallet-option-icon-${WalletName.LUNCDASH}`)).toBeInTheDocument()
    expect(within(lunc).getByText('WalletConnect')).toBeInTheDocument()
    const galaxy = screen.getByRole('button', { name: /^Galaxy Station$/i })
    expect(within(galaxy).getByTestId(`wallet-option-icon-${WalletName.GALAXYSTATION}`)).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /luncdash|galaxy/i })).not.toBeInTheDocument()
  })
})
