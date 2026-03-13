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

const RECONNECT_MAX_RETRIES = 3
const RECONNECT_BASE_DELAY_MS = 600

function isPermanentError(err: unknown): boolean {
  if (!(err instanceof Error)) return false
  const msg = err.message.toLowerCase()
  return msg.includes('rejected') || msg.includes('not installed') || msg.includes('unsupported')
}

async function attemptAutoReconnect(): Promise<void> {
  let saved: string | null
  try {
    saved = localStorage.getItem(WALLET_STORAGE_KEY)
  } catch { return }
  if (!saved) return

  let parsed: { walletName?: string; walletType?: string }
  try {
    parsed = JSON.parse(saved)
  } catch {
    localStorage.removeItem(WALLET_STORAGE_KEY)
    return
  }

  if (
    !parsed ||
    typeof parsed !== 'object' ||
    typeof parsed.walletName !== 'string' ||
    typeof parsed.walletType !== 'string' ||
    !VALID_WALLET_NAMES.has(parsed.walletName) ||
    !VALID_WALLET_TYPES.has(parsed.walletType)
  ) {
    localStorage.removeItem(WALLET_STORAGE_KEY)
    return
  }

  const { walletName, walletType } = parsed as { walletName: WalletName; walletType: WalletType }

  for (let attempt = 0; attempt < RECONNECT_MAX_RETRIES; attempt++) {
    try {
      await useWalletStore.getState().connect(walletName, walletType)
      return
    } catch (err) {
      if (isPermanentError(err)) {
        try { localStorage.removeItem(WALLET_STORAGE_KEY) } catch { /* */ }
        return
      }
      if (attempt < RECONNECT_MAX_RETRIES - 1) {
        await new Promise((r) => setTimeout(r, RECONNECT_BASE_DELAY_MS * (attempt + 1)))
      }
    }
  }
}

if (typeof window !== 'undefined') {
  const reconnect = () => { void attemptAutoReconnect() }

  if (document.readyState === 'complete') {
    reconnect()
  } else {
    window.addEventListener('load', reconnect, { once: true })
  }
}
