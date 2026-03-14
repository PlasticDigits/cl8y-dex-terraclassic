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
              className="w-full flex items-center justify-between p-3 border-2 rounded-none transition-colors shadow-[2px_2px_0_#000]"
              style={{
                borderColor: '#92400e',
                background: '#221c13',
              }}
            >
              <span className="font-medium text-amber-400 uppercase tracking-wide text-sm">Simulated Wallet</span>
              <span className="text-[10px] text-amber-400/60 uppercase tracking-wider font-semibold border border-amber-500/30 px-1.5 py-0.5">
                DEV
              </span>
            </button>
          )}

          {WALLET_OPTIONS.map((option) => (
            <button
              key={option.name}
              onClick={() => void handleConnect(option)}
              onMouseEnter={() => sounds.playHover()}
              disabled={isConnecting}
              className="w-full flex items-center justify-between p-3 border-2 rounded-none transition-colors shadow-[2px_2px_0_#000] hover:translate-x-[1px] hover:translate-y-[1px] hover:shadow-[1px_1px_0_#000] disabled:opacity-50 group"
              style={{
                borderColor: 'rgba(255,255,255,0.2)',
                background: 'var(--surface-1)',
              }}
            >
              <span className="font-medium uppercase tracking-wide text-sm" style={{ color: 'var(--ink)' }}>
                {option.name}
              </span>
              <span
                className="text-[10px] uppercase tracking-wider font-semibold"
                style={{ color: 'var(--ink-subtle)' }}
              >
                {option.connectionLabel}
              </span>
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
