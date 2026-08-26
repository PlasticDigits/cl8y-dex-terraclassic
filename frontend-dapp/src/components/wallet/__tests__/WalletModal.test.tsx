import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, within, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
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
import { useWalletConnectPairingStore } from '@/hooks/useWalletConnectPairingStore'
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
      cancelConnection: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
    useWalletConnectPairingStore.setState({ isOpen: false, payload: null })
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
      cancelConnection: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
    mockSnapshot.mockReturnValue(extensionSnapshot({}))
    useWalletConnectPairingStore.setState({ isOpen: false, payload: null })
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

describe('WalletModal Android Chrome connect (GitLab #554)', () => {
  const androidChrome =
    'Mozilla/5.0 (Linux; Android 16; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36'

  beforeEach(() => {
    vi.clearAllMocks()
    mockUseWalletStore.mockReturnValue({
      connect: vi.fn().mockResolvedValue(undefined),
      connectDev: vi.fn(),
      isConnecting: false,
      error: null,
      cancelConnection: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
    mockSnapshot.mockReturnValue(extensionSnapshot({}))
    useWalletConnectPairingStore.setState({ isOpen: false, payload: null })
    Object.defineProperty(navigator, 'userAgent', { configurable: true, value: androidChrome })
  })

  it('offers Keplr WalletConnect without Install when no window.keplr', () => {
    render(<WalletModal onClose={() => {}} />)
    const keplr = screen.getByRole('button', { name: /^Keplr$/i })
    expect(within(keplr).getByText('WalletConnect')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Keplr, extension not detected/i })).not.toBeInTheDocument()
    const row = keplr.closest('.wallet-option-row')
    expect(row).toBeTruthy()
    expect(within(row as HTMLElement).queryByRole('link', { name: /^install$/i })).not.toBeInTheDocument()
  })

  it('offers Station WalletConnect without Install when station is not injected (GitLab #566)', () => {
    render(<WalletModal onClose={() => {}} />)
    const station = screen.getByRole('button', { name: /^Station$/i })
    expect(within(station).getByText('WalletConnect')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Station, extension not detected/i })).not.toBeInTheDocument()
    const row = station.closest('.wallet-option-row')
    expect(row).toBeTruthy()
    expect(within(row as HTMLElement).queryByRole('link', { name: /^install$/i })).not.toBeInTheDocument()
  })

  it('offers Cosmostation WalletConnect without Install when Cosmostation is not injected (GitLab #566)', () => {
    render(<WalletModal onClose={() => {}} />)
    const cosmo = screen.getByRole('button', { name: /^Cosmostation$/i })
    expect(within(cosmo).getByText('WalletConnect')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Cosmostation, extension not detected/i })).not.toBeInTheDocument()
    const row = cosmo.closest('.wallet-option-row')
    expect(row).toBeTruthy()
    expect(within(row as HTMLElement).queryByRole('link', { name: /^install$/i })).not.toBeInTheDocument()
  })

  it('keeps injected Station as Extension + Ready on mobile (WC-M7, GitLab #566)', () => {
    mockSnapshot.mockReturnValue(extensionSnapshot({ [WalletName.STATION]: true }))
    render(<WalletModal onClose={() => {}} />)
    const station = screen.getByRole('button', { name: /Station, extension detected/i })
    expect(within(station).getByText('Extension')).toBeInTheDocument()
    expect(within(station).getByText('Ready')).toBeInTheDocument()
    const keplr = screen.getByRole('button', { name: /^Keplr$/i })
    expect(within(keplr).getByText('WalletConnect')).toBeInTheDocument()
  })

  it('keeps injected Cosmostation as Extension + Ready on mobile (WC-M7, GitLab #566)', () => {
    mockSnapshot.mockReturnValue(extensionSnapshot({ [WalletName.COSMOSTATION]: true }))
    render(<WalletModal onClose={() => {}} />)
    const cosmo = screen.getByRole('button', { name: /Cosmostation, extension detected/i })
    expect(within(cosmo).getByText('Extension')).toBeInTheDocument()
    expect(within(cosmo).getByText('Ready')).toBeInTheDocument()
  })

  it('still does not list Leap on mobile WalletConnect (GitLab #159 / #566)', () => {
    render(<WalletModal onClose={() => {}} />)
    expect(screen.queryByText(/^leap$/i)).not.toBeInTheDocument()
  })

  it('hides the Connect list while the pairing sheet is open', () => {
    useWalletConnectPairingStore.setState({
      isOpen: true,
      payload: {
        uri: 'wc:00e46b69-d0cc-4b3e-b6a2-cee442f97188@1?bridge=https%3A%2F%2Fwalletconnect.luncdash.com&key=abc',
        name: 'LUNC Dash',
        android: '',
        ios: '',
        isStation: true,
        isLuncDash: true,
      },
    })
    render(<WalletModal onClose={() => {}} />)
    expect(screen.queryByTestId('wallet-connect-modal-portal')).not.toBeInTheDocument()
    expect(screen.queryByText('Connect Wallet')).not.toBeInTheDocument()
  })

  it('shows Cancel while connecting', () => {
    mockUseWalletStore.mockReturnValue({
      connect: vi.fn().mockResolvedValue(undefined),
      connectDev: vi.fn(),
      isConnecting: true,
      error: null,
      cancelConnection: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
    render(<WalletModal onClose={() => {}} />)
    expect(screen.getByTestId('wallet-connect-cancel')).toHaveTextContent('Cancel')
  })
})

describe('WalletModal dismiss (GitLab #672)', () => {
  const desktopChrome =
    'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36'

  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(navigator, 'userAgent', { configurable: true, value: desktopChrome })
    mockUseWalletStore.mockReturnValue({
      connect: vi.fn().mockResolvedValue(undefined),
      connectDev: vi.fn(),
      isConnecting: false,
      error: null,
      cancelConnection: vi.fn(),
    } as ReturnType<typeof useWalletStore>)
    mockSnapshot.mockReturnValue(extensionSnapshot({}))
    useWalletConnectPairingStore.setState({ isOpen: false, payload: null })
  })

  it('shows a labeled Close connect wallet control (D1)', () => {
    render(<WalletModal onClose={() => {}} />)
    const close = screen.getByRole('button', { name: 'Close connect wallet' })
    expect(close).toHaveTextContent('Close')
    expect(close).toBeVisible()
  })

  it('closes on header Close and on dimmed backdrop (D2)', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(<WalletModal onClose={onClose} />)
    await user.click(screen.getByRole('button', { name: 'Close connect wallet' }))
    expect(onClose).toHaveBeenCalledTimes(1)
    onClose.mockClear()
    fireEvent.click(screen.getByTestId('modal-backdrop'))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('does not close when a wallet row or Install is clicked (D4)', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    const connect = vi.fn().mockResolvedValue(undefined)
    const store = {
      connect,
      connectDev: vi.fn(),
      isConnecting: false,
      error: null,
      cancelConnection: vi.fn(),
      address: null,
    }
    mockUseWalletStore.mockReturnValue(store as ReturnType<typeof useWalletStore>)
    mockUseWalletStore.getState = () => store as ReturnType<typeof useWalletStore.getState>
    render(<WalletModal onClose={onClose} />)
    await user.click(screen.getByRole('button', { name: /^Galaxy Station$/i }))
    expect(onClose).not.toHaveBeenCalled()
    fireEvent.click(screen.getAllByRole('link', { name: /^install$/i })[0])
    expect(onClose).not.toHaveBeenCalled()
  })

  it('cancels an in-flight connect instead of only hiding the dialog (D6)', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    const cancelConnection = vi.fn()
    mockUseWalletStore.mockReturnValue({
      connect: vi.fn().mockResolvedValue(undefined),
      connectDev: vi.fn(),
      isConnecting: true,
      error: null,
      cancelConnection,
    } as ReturnType<typeof useWalletStore>)
    render(<WalletModal onClose={onClose} />)
    await user.click(screen.getByRole('button', { name: 'Close connect wallet' }))
    expect(cancelConnection).toHaveBeenCalledTimes(1)
    expect(onClose).not.toHaveBeenCalled()
  })
})
