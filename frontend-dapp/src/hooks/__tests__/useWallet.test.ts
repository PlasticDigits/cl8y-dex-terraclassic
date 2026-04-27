import { describe, it, expect, vi, beforeEach } from 'vitest'
import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'

vi.mock('@/services/terraclassic/wallet', () => ({
  connectTerraWallet: vi.fn(),
  disconnectTerraWallet: vi.fn(),
  registerConnectedWallet: vi.fn(),
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
    expect(state.error).toBe('User rejected')
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
})
