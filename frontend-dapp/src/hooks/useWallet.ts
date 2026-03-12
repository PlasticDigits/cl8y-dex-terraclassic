import { create } from 'zustand'
import { connectTerraWallet, disconnectTerraWallet, registerConnectedWallet } from '@/services/terraclassic/wallet'
import { createDevTerraWallet, DEV_TERRA_ADDRESS } from '@/services/terraclassic/devWallet'
import { DEV_MODE } from '@/utils/constants'
import type { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'

const WALLET_STORAGE_KEY = 'cl8y_wallet_connection'

interface WalletState {
  address: string | null
  walletType: string | null
  isConnecting: boolean
  error: string | null
  connect: (walletName: WalletName, walletType: WalletType) => Promise<void>
  connectDev: () => void
  disconnect: () => Promise<void>
}

export const useWalletStore = create<WalletState>((set) => ({
  address: null,
  walletType: null,
  isConnecting: false,
  error: null,
  connect: async (walletName, walletType) => {
    set({ isConnecting: true, error: null })
    try {
      const result = await connectTerraWallet(walletName, walletType)
      try {
        localStorage.setItem(WALLET_STORAGE_KEY, JSON.stringify({ walletName, walletType }))
      } catch { /* storage unavailable */ }
      set({ address: result.address, walletType: result.walletType, isConnecting: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Connection failed', isConnecting: false })
    }
  },
  connectDev: () => {
    if (!DEV_MODE) return
    const devWallet = createDevTerraWallet()
    registerConnectedWallet(devWallet)
    set({ address: DEV_TERRA_ADDRESS, walletType: 'simulated', error: null })
  },
  disconnect: async () => {
    await disconnectTerraWallet()
    try {
      localStorage.removeItem(WALLET_STORAGE_KEY)
    } catch { /* storage unavailable */ }
    set({ address: null, walletType: null })
  },
}))

if (typeof window !== 'undefined') {
  try {
    const saved = localStorage.getItem(WALLET_STORAGE_KEY)
    if (saved) {
      const { walletName, walletType } = JSON.parse(saved) as { walletName: WalletName; walletType: WalletType }
      useWalletStore.getState().connect(walletName, walletType).catch(() => {
        localStorage.removeItem(WALLET_STORAGE_KEY)
      })
    }
  } catch { /* ignore parse / storage errors */ }
}
