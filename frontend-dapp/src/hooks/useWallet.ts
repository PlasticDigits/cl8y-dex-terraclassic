import { create } from 'zustand'
import { connectTerraWallet, disconnectTerraWallet, registerConnectedWallet } from '@/services/terraclassic/wallet'
import { createDevTerraWallet, DEV_TERRA_ADDRESS } from '@/services/terraclassic/devWallet'
import { DEV_MODE } from '@/utils/constants'
import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'

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

const VALID_WALLET_NAMES = new Set(Object.values(WalletName as Record<string, string>))
const VALID_WALLET_TYPES = new Set(Object.values(WalletType as Record<string, string>))

if (typeof window !== 'undefined') {
  try {
    const saved = localStorage.getItem(WALLET_STORAGE_KEY)
    if (saved) {
      const parsed = JSON.parse(saved)
      if (
        parsed &&
        typeof parsed === 'object' &&
        typeof parsed.walletName === 'string' &&
        typeof parsed.walletType === 'string' &&
        VALID_WALLET_NAMES.has(parsed.walletName) &&
        VALID_WALLET_TYPES.has(parsed.walletType)
      ) {
        useWalletStore.getState().connect(parsed.walletName as WalletName, parsed.walletType as WalletType).catch(() => {
          localStorage.removeItem(WALLET_STORAGE_KEY)
        })
      } else {
        localStorage.removeItem(WALLET_STORAGE_KEY)
      }
    }
  } catch { /* ignore parse / storage errors */ }
}
