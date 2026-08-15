import { CopyButton } from '@/components/ui/CopyButton'
import { Modal } from '@/components/ui'
import { useWalletConnectPairingStore } from '@/hooks/useWalletConnectPairingStore'
import { sounds } from '@/lib/sounds'
import { buildWalletConnectDeepLinks, isAllowedWalletConnectDeepLink } from '@/utils/walletConnectPairing'

export default function WalletConnectPairingModal() {
  const { isOpen, payload, close } = useWalletConnectPairingStore()

  if (!isOpen || !payload) return null

  const links = buildWalletConnectDeepLinks(payload, payload.uri)

  return (
    <Modal isOpen={true} onClose={close} title={`Connect ${payload.name}`}>
      <div className="walletconnect-pairing px-6 py-4" data-testid="walletconnect-pairing-modal">
        <p className="mb-4 text-sm" style={{ color: 'var(--ink-subtle)' }}>
          Open your wallet, then return here.
        </p>
        <div className="flex flex-col gap-2">
          {links.map((link) => (
            <a
              key={link.id}
              className={
                link.id === 'wallet' ? 'btn-primary walletconnect-pairing-link' : 'btn-muted walletconnect-pairing-link'
              }
              href={isAllowedWalletConnectDeepLink(link.href) ? link.href : undefined}
              data-testid={`walletconnect-pairing-${link.id}`}
              onClick={(event) => {
                if (!isAllowedWalletConnectDeepLink(link.href)) {
                  event.preventDefault()
                  return
                }
                sounds.playButtonPress()
              }}
            >
              {link.label}
            </a>
          ))}
          <CopyButton
            text={payload.uri}
            ariaLabel="Copy pairing link"
            buttonLabel="Copy pairing link"
            data-testid="walletconnect-pairing-copy"
          />
        </div>
      </div>
    </Modal>
  )
}
