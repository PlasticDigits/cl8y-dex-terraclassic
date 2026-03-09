import { create } from 'zustand'
import { connectTerraWallet, disconnectTerraWallet } from '@/services/terraclassic/wallet'
import { createDevTerraWallet, DEV_TERRA_ADDRESS } from '@/services/terraclassic/devWallet'
import { DEV_MODE } from '@/utils/constants'
import type { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'

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
      set({ address: result.address, walletType: result.walletType, isConnecting: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Connection failed', isConnecting: false })
    }
  },
  connectDev: () => {
    if (!DEV_MODE) return
    createDevTerraWallet()
    set({ address: DEV_TERRA_ADDRESS, walletType: 'simulated', error: null })
  },
  disconnect: async () => {
    await disconnectTerraWallet()
    set({ address: null, walletType: null })
  },
}))
