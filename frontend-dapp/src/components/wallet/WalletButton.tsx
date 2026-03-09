import { useState } from 'react'
import { createPortal } from 'react-dom'
import { useWalletStore } from '@/hooks/useWallet'
import WalletModal from './WalletModal'

function truncateAddress(addr: string): string {
  return `${addr.slice(0, 8)}...${addr.slice(-4)}`
}

export default function WalletButton() {
  const { address, disconnect } = useWalletStore()
  const [showModal, setShowModal] = useState(false)

  if (address) {
    return (
      <div className="flex items-center gap-3">
        <span className="text-sm text-dex-accent font-mono bg-dex-bg/50 px-3 py-1.5 rounded-lg border border-dex-border">
          {truncateAddress(address)}
        </span>
        <button
          onClick={() => void disconnect()}
          className="text-sm text-gray-400 hover:text-red-400 transition-colors px-3 py-1.5 rounded-lg border border-dex-border hover:border-red-500/50"
        >
          Disconnect
        </button>
      </div>
    )
  }

  return (
    <>
      <button
        onClick={() => setShowModal(true)}
        className="bg-dex-accent hover:bg-dex-accent/80 text-dex-bg font-semibold text-sm px-5 py-2 rounded-xl transition-colors"
      >
        Connect Wallet
      </button>
      {showModal && createPortal(
        <WalletModal onClose={() => setShowModal(false)} />,
        document.body
      )}
    </>
  )
}
