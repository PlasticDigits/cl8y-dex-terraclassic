import { CopyButton } from '@/components/ui/CopyButton'
import { sounds } from '@/lib/sounds'
import { getExplorerAddressUrl, isSafeExplorerHref } from '@/utils/terraExplorer'

const menuIconClass = 'w-4 h-4 shrink-0'

const EXPLORER_MENU_ICON = (
  <svg className={menuIconClass} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
    />
  </svg>
)

const SWITCH_WALLET_ICON = (
  <svg className={menuIconClass} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"
    />
  </svg>
)

type WalletDropdownMenuItemsProps = {
  address: string
  onClose: () => void
  onSwitchWallet: () => void
}

/** Standard wallet menu rows between the AddressRow header and profile/disconnect (GitLab #185). */
export function WalletDropdownMenuItems({ address, onClose, onSwitchWallet }: WalletDropdownMenuItemsProps) {
  const explorerUrl = getExplorerAddressUrl(address)
  const safeExplorerUrl = isSafeExplorerHref(explorerUrl) ? explorerUrl : null

  return (
    <>
      <CopyButton
        text={address}
        ariaLabel="Copy wallet address"
        menuLabel="Copy address"
        data-testid="wallet-menu-copy-address"
      />
      {safeExplorerUrl ? (
        <a
          role="menuitem"
          href={safeExplorerUrl}
          target="_blank"
          rel="noopener noreferrer"
          onClick={() => {
            sounds.playButtonPress()
            onClose()
          }}
          className="wallet-menu-item"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="wallet-menu-view-explorer"
        >
          {EXPLORER_MENU_ICON}
          View on explorer
        </a>
      ) : null}
      <button
        type="button"
        role="menuitem"
        onClick={() => {
          sounds.playButtonPress()
          onSwitchWallet()
          onClose()
        }}
        className="wallet-menu-item"
        style={{ color: 'var(--ink-dim)' }}
        data-testid="wallet-menu-switch-wallet"
      >
        {SWITCH_WALLET_ICON}
        Switch wallet
      </button>
    </>
  )
}
