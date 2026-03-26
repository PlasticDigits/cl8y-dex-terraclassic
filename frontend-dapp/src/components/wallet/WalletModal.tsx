import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { useWalletStore } from '@/hooks/useWallet'
import { DEV_MODE } from '@/utils/constants'
import { Modal } from '@/components/ui'
import { sounds } from '@/lib/sounds'

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
  {
    name: 'Cosmostation',
    walletName: WalletName.COSMOSTATION,
    walletType: WalletType.EXTENSION,
    connectionLabel: 'Extension',
  },
  {
    name: 'LuncDash',
    walletName: WalletName.LUNCDASH,
    walletType: WalletType.WALLETCONNECT,
    connectionLabel: 'WalletConnect',
  },
  {
    name: 'Galaxy Station',
    walletName: WalletName.GALAXYSTATION,
    walletType: WalletType.WALLETCONNECT,
    connectionLabel: 'WalletConnect',
  },
]

interface WalletModalProps {
  onClose: () => void
}

export default function WalletModal({ onClose }: WalletModalProps) {
  const { connect, connectDev, isConnecting, error } = useWalletStore()

  async function handleConnect(option: WalletOption) {
    sounds.playButtonPress()
    await connect(option.walletName, option.walletType)
    if (!useWalletStore.getState().error) {
      sounds.playSuccess()
      onClose()
    } else {
      sounds.playError()
    }
  }

  function handleDevConnect() {
    sounds.playButtonPress()
    connectDev()
    onClose()
  }

  return (
    <Modal isOpen={true} onClose={onClose} title="Connect Wallet">
      <div className="px-6 py-4">
        {error && <div className="alert-error mb-4">{error}</div>}

        <div className="space-y-2">
          {DEV_MODE && (
            <button
              onClick={handleDevConnect}
              onMouseEnter={() => sounds.playHover()}
              className="wallet-option-card wallet-option-card-dev"
            >
              <span className="font-medium uppercase tracking-wide text-sm" style={{ color: '#ffd28d' }}>
                Simulated Wallet
              </span>
              <span className="wallet-option-badge wallet-option-badge-dev">DEV</span>
            </button>
          )}

          {WALLET_OPTIONS.map((option) => (
            <button
              key={option.name}
              onClick={() => void handleConnect(option)}
              onMouseEnter={() => sounds.playHover()}
              disabled={isConnecting}
              className="wallet-option-card disabled:opacity-50"
            >
              <span className="font-medium uppercase tracking-wide text-sm" style={{ color: 'var(--ink)' }}>
                {option.name}
              </span>
              <span className="wallet-option-badge">{option.connectionLabel}</span>
            </button>
          ))}
        </div>

        {isConnecting && (
          <div className="mt-4 text-center text-sm uppercase tracking-wide" style={{ color: 'var(--ink-subtle)' }}>
            Connecting...
          </div>
        )}
      </div>
    </Modal>
  )
}
