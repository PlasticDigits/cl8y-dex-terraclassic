import { describe, it, expect, vi, beforeEach } from 'vitest'
import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { humanizeUserFacingError } from '@/utils/humanizeUserFacingError'

vi.mock('@/services/terraclassic/wallet', () => ({
  connectTerraWallet: vi.fn(),
  disconnectTerraWallet: vi.fn(),
  registerConnectedWallet: vi.fn(),
  abortPendingTerraWalletConnect: vi.fn(),
}))
vi.mock('@/services/terraclassic/devWallet', () => ({
  createDevTerraWallet: vi.fn(() => ({ address: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v' })),
}))
vi.mock('@/utils/constants', () => ({
  DEV_MODE: true,
}))

const WALLET_STORAGE_KEY = 'cl8y_wallet_connection'

const { connectTerraWallet, disconnectTerraWallet } = await import('@/services/terraclassic/wallet')
const { useWalletStore } = await import('../useWallet')

describe('useWalletStore', () => {
  beforeEach(() => {
    useWalletStore.setState({
      address: null,
      walletType: null,
      isConnecting: false,
      error: null,
      walletModalOpen: false,
    })
    vi.mocked(connectTerraWallet).mockReset()
    vi.mocked(disconnectTerraWallet).mockReset()
    localStorage.clear()
  })

  it('initial state - address is null, walletType is null, isConnecting is false, error is null', () => {
    const state = useWalletStore.getState()
    expect(state.address).toBeNull()
    expect(state.walletType).toBeNull()
    expect(state.isConnecting).toBe(false)
    expect(state.error).toBeNull()
  })

  it('connect - successful connection sets address and walletType, clears error', async () => {
    vi.mocked(connectTerraWallet).mockResolvedValueOnce({
      address: 'terra1abc123',
      walletType: 'station',
      connectionType: WalletType.EXTENSION,
    })

    useWalletStore.setState({ error: 'previous error' })
    await useWalletStore.getState().connect(WalletName.STATION, WalletType.EXTENSION)

    const state = useWalletStore.getState()
    expect(state.address).toBe('terra1abc123')
    expect(state.walletType).toBe('station')
    expect(state.isConnecting).toBe(false)
    expect(state.error).toBeNull()
    expect(connectTerraWallet).toHaveBeenCalledWith(WalletName.STATION, WalletType.EXTENSION)
  })

  it('connect - failed connection sets error, clears isConnecting, rethrows', async () => {
    vi.mocked(connectTerraWallet).mockRejectedValueOnce(new Error('User rejected'))

    await expect(useWalletStore.getState().connect(WalletName.STATION, WalletType.EXTENSION)).rejects.toThrow(
      'User rejected'
    )

    const state = useWalletStore.getState()
    expect(state.address).toBeNull()
    expect(state.walletType).toBeNull()
    expect(state.isConnecting).toBe(false)
    expect(state.error).toBe(humanizeUserFacingError('User rejected'))
  })

  it('connectDev - sets address from createDevTerraWallet, walletType to simulated', () => {
    useWalletStore.getState().connectDev()

    const state = useWalletStore.getState()
    expect(state.address).toBe('terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v')
    expect(state.walletType).toBe('simulated')
  })

  it('disconnect - clears address and walletType', async () => {
    useWalletStore.setState({ address: 'terra1abc123', walletType: 'station' })
    vi.mocked(disconnectTerraWallet).mockResolvedValueOnce(undefined)

    await useWalletStore.getState().disconnect()

    const state = useWalletStore.getState()
    expect(state.address).toBeNull()
    expect(state.walletType).toBeNull()
    expect(disconnectTerraWallet).toHaveBeenCalled()
  })

  it('connect saves to localStorage', async () => {
    vi.mocked(connectTerraWallet).mockResolvedValueOnce({
      address: 'terra1abc123',
      walletType: 'keplr',
      connectionType: WalletType.EXTENSION,
    })

    await useWalletStore.getState().connect(WalletName.KEPLR, WalletType.EXTENSION)

    const saved = localStorage.getItem(WALLET_STORAGE_KEY)
    expect(saved).not.toBeNull()
    const parsed = JSON.parse(saved!)
    expect(parsed).toEqual({
      walletName: WalletName.KEPLR,
      walletType: WalletType.EXTENSION,
    })
  })

  it('disconnect removes from localStorage', async () => {
    localStorage.setItem(
      WALLET_STORAGE_KEY,
      JSON.stringify({ walletName: WalletName.STATION, walletType: WalletType.EXTENSION })
    )
    useWalletStore.setState({ address: 'terra1abc123', walletType: 'station' })
    vi.mocked(disconnectTerraWallet).mockResolvedValueOnce(undefined)

    await useWalletStore.getState().disconnect()

    expect(localStorage.getItem(WALLET_STORAGE_KEY)).toBeNull()
  })

  it('clears persisted Leap session on load (GitLab #159)', async () => {
    vi.resetModules()
    localStorage.setItem(
      WALLET_STORAGE_KEY,
      JSON.stringify({ walletName: WalletName.LEAP, walletType: WalletType.EXTENSION })
    )
    await import('../useWallet')
    await new Promise((r) => setTimeout(r, 0))
    expect(localStorage.getItem(WALLET_STORAGE_KEY)).toBeNull()
    expect(connectTerraWallet).not.toHaveBeenCalled()
  })

  it('cancelConnection clears isConnecting and ignores a late WC session (GitLab #554)', async () => {
    let resolveConnect!: (value: { address: string; walletType: 'keplr'; connectionType: WalletType }) => void
    vi.mocked(connectTerraWallet).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveConnect = resolve
        })
    )

    const pending = useWalletStore.getState().connect(WalletName.KEPLR, WalletType.WALLETCONNECT)
    expect(useWalletStore.getState().isConnecting).toBe(true)

    useWalletStore.getState().cancelConnection()
    expect(useWalletStore.getState().isConnecting).toBe(false)
    expect(useWalletStore.getState().address).toBeNull()

    resolveConnect({
      address: 'terra1lateconnect',
      walletType: 'keplr',
      connectionType: WalletType.WALLETCONNECT,
    })
    await pending
    expect(useWalletStore.getState().address).toBeNull()
    expect(useWalletStore.getState().isConnecting).toBe(false)
    expect(disconnectTerraWallet).toHaveBeenCalled()
  })

  it('timeout error from connectTerraWallet clears isConnecting and sets a retail error (GitLab #554)', async () => {
    vi.mocked(connectTerraWallet).mockRejectedValueOnce(new Error("Wallet didn't respond. Try again."))

    await expect(useWalletStore.getState().connect(WalletName.LUNCDASH, WalletType.WALLETCONNECT)).rejects.toThrow(
      /didn't respond/i
    )
    expect(useWalletStore.getState().isConnecting).toBe(false)
    expect(useWalletStore.getState().error).toMatch(/didn't respond/i)
    expect(useWalletStore.getState().address).toBeNull()
  })

  it('closeWalletModal hides the dialog without connecting (GitLab #672)', () => {
    useWalletStore.setState({ walletModalOpen: true, address: null, isConnecting: false })
    useWalletStore.getState().closeWalletModal()
    expect(useWalletStore.getState().walletModalOpen).toBe(false)
    expect(useWalletStore.getState().address).toBeNull()
    expect(localStorage.getItem(WALLET_STORAGE_KEY)).toBeNull()
  })

  it('closeWalletModal while connecting cancels and does not attach a session (GitLab #672 D6)', async () => {
    let resolveConnect!: (value: { address: string; walletType: 'keplr'; connectionType: WalletType }) => void
    vi.mocked(connectTerraWallet).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveConnect = resolve
        })
    )

    const pending = useWalletStore.getState().connect(WalletName.KEPLR, WalletType.WALLETCONNECT)
    useWalletStore.setState({ walletModalOpen: true })
    expect(useWalletStore.getState().isConnecting).toBe(true)

    useWalletStore.getState().closeWalletModal()
    expect(useWalletStore.getState().isConnecting).toBe(false)
    expect(useWalletStore.getState().walletModalOpen).toBe(false)
    expect(useWalletStore.getState().address).toBeNull()
    expect(localStorage.getItem(WALLET_STORAGE_KEY)).toBeNull()

    resolveConnect({
      address: 'terra1lateconnect',
      walletType: 'keplr',
      connectionType: WalletType.WALLETCONNECT,
    })
    await pending
    expect(useWalletStore.getState().address).toBeNull()
  })

  it('openWalletModal only opens and does not toggle closed (GitLab #672)', () => {
    useWalletStore.setState({ walletModalOpen: true })
    useWalletStore.getState().openWalletModal()
    expect(useWalletStore.getState().walletModalOpen).toBe(true)
  })
})
