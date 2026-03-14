import { useState } from 'react'
import { createPortal } from 'react-dom'
import { useWalletStore } from '@/hooks/useWallet'
import { sounds } from '@/lib/sounds'
import { shortenAddress } from '@/utils/tokenDisplay'
import WalletModal from './WalletModal'

export default function WalletButton() {
  const { address, isConnecting, disconnect } = useWalletStore()
  const [showModal, setShowModal] = useState(false)
  const [showDropdown, setShowDropdown] = useState(false)

  if (address) {
    return (
      <div className="relative">
        <button
          onClick={() => {
            sounds.playButtonPress()
            setShowDropdown(!showDropdown)
          }}
          aria-haspopup="true"
          aria-expanded={showDropdown}
          className="flex items-center gap-2 sm:gap-3 px-3 sm:px-4 py-2 glass border-2 border-white/30 hover:border-white/60 rounded-none transition-all group shadow-[3px_3px_0_#000]"
        >
          <div className="text-right hidden sm:block">
            <p className="text-xs" style={{ color: 'var(--ink-subtle)' }}>
              {shortenAddress(address, 6, 6)}
            </p>
          </div>
          <div className="sm:hidden text-xs font-mono font-medium" style={{ color: 'var(--ink)' }}>
            {shortenAddress(address, 4, 4)}
          </div>
          <div className="w-8 h-8 shrink-0 flex items-center justify-center overflow-hidden rounded-sm bg-black/90 p-1 border-2 border-black shadow-[2px_2px_0_#000]">
            <img src="/chains/terraclassic-icon.png" alt="Terra Classic" className="h-full w-full object-contain" />
          </div>
        </button>

        {showDropdown && (
          <>
            <div className="fixed inset-0 z-40" onClick={() => setShowDropdown(false)} />
            <div
              role="menu"
              className="absolute right-0 mt-2 w-48 glass border-2 border-white/35 rounded-none shadow-[4px_4px_0_#000] overflow-hidden z-50 animate-fade-in-up"
              style={{ animationDuration: '0.2s' }}
            >
              <div className="p-2">
                <div className="px-3 py-2 sm:hidden">
                  <p className="text-xs" style={{ color: 'var(--ink-subtle)' }}>
                    {shortenAddress(address, 8, 8)}
                  </p>
                </div>
                <button
                  role="menuitem"
                  onClick={() => {
                    sounds.playButtonPress()
                    void disconnect()
                    setShowDropdown(false)
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2.5 text-left text-sm hover:bg-white/5 hover:text-red-400 rounded-lg transition-colors"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
                    />
                  </svg>
                  Disconnect
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    )
  }

  return (
    <>
      <button
        onClick={() => {
          sounds.playButtonPress()
          setShowModal(true)
        }}
        disabled={isConnecting}
        className="btn-primary disabled:opacity-60 disabled:cursor-not-allowed"
      >
        <span className="flex items-center gap-2">
          {isConnecting ? (
            <>
              <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
              </svg>
              <span className="hidden sm:inline">Connecting...</span>
            </>
          ) : (
            <>
              <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded bg-black p-0.5">
                <img src="/chains/terraclassic-icon.png" alt="" className="h-full w-full object-contain" />
              </span>
              <span className="hidden sm:inline">CONNECT TC</span>
              <span className="sm:hidden">TC</span>
            </>
          )}
        </span>
      </button>
      {showModal && createPortal(<WalletModal onClose={() => setShowModal(false)} />, document.body)}
    </>
  )
}
