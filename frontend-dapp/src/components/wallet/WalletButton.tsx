import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { Link } from 'react-router-dom'
import { useWalletStore } from '@/hooks/useWallet'
import { sounds } from '@/lib/sounds'
import { AddressRow } from '@/components/ui/AddressRow'
import { shortenAddress } from '@/utils/tokenDisplay'
import { DEFAULT_NETWORK, NETWORKS } from '@/utils/constants'
import { getNetworkBadgeCopy, getTerraChainLogoPath } from '@/utils/networkDisplay'
import { WalletChipNetworkIndicator } from './WalletChipNetworkIndicator'
import { WalletDropdownMenuItems } from './WalletDropdownMenuItems'
import { WalletLuncBalance } from './WalletLuncBalance'
import WalletModal from './WalletModal'

export default function WalletButton() {
  const { address, isConnecting, disconnect, walletModalOpen, setWalletModalOpen, closeWalletModal } = useWalletStore()
  const [showDropdown, setShowDropdown] = useState(false)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)
  const chainLogoPath = getTerraChainLogoPath(NETWORKS[DEFAULT_NETWORK].terra.chainId)
  const { shortLabel: networkShortLabel } = getNetworkBadgeCopy()

  const closeWalletMenu = () => {
    setShowDropdown(false)
    requestAnimationFrame(() => triggerRef.current?.focus())
  }

  const handleSwitchWallet = () => {
    void disconnect().then(() => setWalletModalOpen(true))
  }

  useEffect(() => {
    if (!showDropdown) return

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closeWalletMenu()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [showDropdown])

  useEffect(() => {
    if (!showDropdown) return
    const frame = requestAnimationFrame(() => {
      const menu = menuRef.current
      const firstItem = menu?.querySelector<HTMLElement>('[role="menuitem"]')
      ;(firstItem ?? menu)?.focus()
    })
    return () => cancelAnimationFrame(frame)
  }, [showDropdown])

  if (address) {
    return (
      <>
        <div className="wallet-dropdown-wrap">
          <button
            ref={triggerRef}
            type="button"
            onClick={() => {
              sounds.playButtonPress()
              setShowDropdown(!showDropdown)
            }}
            aria-haspopup="menu"
            aria-expanded={showDropdown}
            aria-label={`Connected wallet on ${networkShortLabel}`}
            className="wallet-trigger wallet-trigger-connected"
          >
            <div className="text-left hidden sm:block min-w-0 max-w-[9.5rem]">
              <p className="text-[10px] uppercase tracking-[0.12em]" style={{ color: 'var(--ink-subtle)' }}>
                Wallet
              </p>
              <WalletLuncBalance address={address} className="block max-w-full" />
              <p className="text-[10px] font-mono truncate" style={{ color: 'var(--ink-dim)' }}>
                {shortenAddress(address, 6, 6)}
              </p>
            </div>
            <div className="sm:hidden text-right min-w-0 max-w-[5.5rem]">
              <WalletLuncBalance address={address} className="block max-w-full" />
              <p className="text-xs font-mono font-medium truncate" style={{ color: 'var(--ink)' }}>
                {shortenAddress(address, 4, 4)}
              </p>
            </div>
            <WalletChipNetworkIndicator />
          </button>

          {showDropdown && (
            <>
              <button
                type="button"
                aria-label="Close wallet menu"
                className="app-menu-dismiss"
                onClick={closeWalletMenu}
              />
              <div className="wallet-menu animate-fade-in-up" style={{ animationDuration: '0.2s' }}>
                <div className="px-3 py-2 border-b border-white/10 space-y-1 min-w-0">
                  <WalletLuncBalance address={address} />
                  <AddressRow
                    address={address}
                    showFull
                    className="text-xs"
                    copyAriaLabel="Copy wallet address"
                    explorerAriaLabel="View wallet address on explorer"
                    data-testid="wallet-menu-address-row"
                  />
                </div>
                <div ref={menuRef} role="menu" tabIndex={-1}>
                  <WalletDropdownMenuItems
                    address={address}
                    onClose={closeWalletMenu}
                    onSwitchWallet={handleSwitchWallet}
                  />
                  <Link
                    role="menuitem"
                    to="/portfolio"
                    onClick={() => {
                      sounds.playButtonPress()
                      closeWalletMenu()
                    }}
                    className="wallet-menu-item"
                    style={{ color: 'var(--ink-dim)' }}
                  >
                    <svg className="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
                      />
                    </svg>
                    My Portfolio
                  </Link>
                  <Link
                    role="menuitem"
                    to={`/trader/${address}`}
                    onClick={() => {
                      sounds.playButtonPress()
                      closeWalletMenu()
                    }}
                    className="wallet-menu-item"
                    style={{ color: 'var(--ink-dim)' }}
                  >
                    <svg className="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
                      />
                    </svg>
                    Trader profile
                  </Link>
                  <button
                    type="button"
                    role="menuitem"
                    onClick={() => {
                      sounds.playButtonPress()
                      void disconnect()
                      closeWalletMenu()
                    }}
                    className="wallet-menu-item"
                    style={{ color: 'var(--ink-dim)' }}
                  >
                    <svg className="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
        {walletModalOpen && createPortal(<WalletModal onClose={closeWalletModal} />, document.body)}
      </>
    )
  }

  return (
    <>
      <button
        type="button"
        onClick={() => {
          sounds.playButtonPress()
          if (walletModalOpen || isConnecting) {
            closeWalletModal()
            return
          }
          setWalletModalOpen(true)
        }}
        aria-label={isConnecting ? 'Cancel connecting' : 'Connect wallet'}
        aria-haspopup="dialog"
        aria-expanded={walletModalOpen}
        className="btn-primary !px-3 !py-2 sm:!px-4 disabled:opacity-60 disabled:cursor-not-allowed"
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
              <span>Cancel</span>
            </>
          ) : (
            <>
              <span className="wallet-trigger-icon">
                <img src={chainLogoPath} alt="" className="h-full w-full object-contain" />
              </span>
              <span className="hidden sm:inline">Connect Wallet</span>
              <span className="sm:hidden">Connect</span>
            </>
          )}
        </span>
      </button>
      {walletModalOpen && createPortal(<WalletModal onClose={closeWalletModal} />, document.body)}
    </>
  )
}
