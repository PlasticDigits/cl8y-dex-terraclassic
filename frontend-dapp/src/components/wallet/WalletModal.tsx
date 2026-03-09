import { motion } from 'framer-motion'
import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { useWalletStore } from '@/hooks/useWallet'
import { DEV_MODE } from '@/utils/constants'

interface WalletOption {
  name: string
  walletName: WalletName
  walletType: WalletType
  connectionLabel: string
}

const WALLET_OPTIONS: WalletOption[] = [
  { name: 'Station', walletName: WalletName.STATION, walletType: WalletType.EXTENSION, connectionLabel: 'Extension' },
  { name: 'Keplr', walletName: WalletName.KEPLR, walletType: WalletType.EXTENSION, connectionLabel: 'Extension' },
  { name: 'Leap', walletName: WalletName.LEAP, walletType: WalletType.EXTENSION, connectionLabel: 'Extension' },
  { name: 'Cosmostation', walletName: WalletName.COSMOSTATION, walletType: WalletType.EXTENSION, connectionLabel: 'Extension' },
  { name: 'LuncDash', walletName: WalletName.LUNCDASH, walletType: WalletType.WALLETCONNECT, connectionLabel: 'WalletConnect' },
  { name: 'Galaxy Station', walletName: WalletName.GALAXYSTATION, walletType: WalletType.WALLETCONNECT, connectionLabel: 'WalletConnect' },
]

interface WalletModalProps {
  onClose: () => void
}

export default function WalletModal({ onClose }: WalletModalProps) {
  const { connect, connectDev, isConnecting, error } = useWalletStore()

  async function handleConnect(option: WalletOption) {
    await connect(option.walletName, option.walletType)
    if (!useWalletStore.getState().error) {
      onClose()
    }
  }

  function handleDevConnect() {
    connectDev()
    onClose()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <motion.div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        onClick={onClose}
      />
      <motion.div
        className="relative bg-dex-card border border-dex-border rounded-2xl p-6 w-full max-w-sm shadow-2xl"
        initial={{ opacity: 0, scale: 0.95, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        transition={{ type: 'spring', damping: 25, stiffness: 300 }}
      >
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-lg font-semibold">Connect Wallet</h3>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors text-xl leading-none"
          >
            &times;
          </button>
        </div>

        {error && (
          <div className="mb-4 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
            {error}
          </div>
        )}

        <div className="space-y-2">
          {DEV_MODE && (
            <button
              onClick={handleDevConnect}
              className="w-full flex items-center justify-between p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 hover:bg-amber-500/20 transition-colors"
            >
              <span className="font-medium text-amber-400">Simulated Wallet</span>
              <span className="text-xs text-amber-400/60">DEV</span>
            </button>
          )}

          {WALLET_OPTIONS.map((option) => (
            <button
              key={option.name}
              onClick={() => void handleConnect(option)}
              disabled={isConnecting}
              className="w-full flex items-center justify-between p-3 rounded-xl hover:bg-white/5 border border-transparent hover:border-dex-border transition-colors disabled:opacity-50 group"
            >
              <span className="font-medium">{option.name}</span>
              <span className="text-xs text-gray-500 group-hover:text-gray-400">
                {option.connectionLabel}
              </span>
            </button>
          ))}
        </div>

        {isConnecting && (
          <div className="mt-4 text-center text-sm text-gray-400">
            Connecting...
          </div>
        )}
      </motion.div>
    </div>
  )
}
