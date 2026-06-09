import { Modal } from '@/components/ui'
import { sounds } from '@/lib/sounds'

export interface ExpertModeModalProps {
  isOpen: boolean
  onClose: () => void
  onEnable: () => void
}

export function ExpertModeModal({ isOpen, onClose, onEnable }: ExpertModeModalProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Enable Expert Mode" panelClassName="max-w-md">
      <div className="space-y-4 text-sm leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
        <p>
          Expert Mode disables the automatic block on swaps with very high expected slippage (above{' '}
          <strong style={{ color: 'var(--ink)' }}>30%</strong> vs best-route token prices).
        </p>
        <p>
          High slippage often means the quoted route exploits thin or mispriced pools. You may receive far fewer tokens
          than the fair cross-rate, or a quote that looks too good to be true on multi-hop paths.
        </p>
        <p>
          Expert Mode does <strong style={{ color: 'var(--ink)' }}>not</strong> override your Settings slippage
          tolerance or per-hop max-spread checks — those still block when on-chain spread would exceed your limit.
        </p>
        <p className="font-semibold" style={{ color: 'var(--color-warning, #f59e0b)' }}>
          Only enable Expert Mode if you understand the execution risk.
        </p>
        <div className="flex flex-col sm:flex-row gap-2 pt-2">
          <button
            type="button"
            className="btn-primary flex-1"
            onClick={() => {
              sounds.playButtonPress()
              onEnable()
            }}
          >
            Enable Expert Mode
          </button>
          <button
            type="button"
            className="btn-muted flex-1"
            onClick={() => {
              sounds.playButtonPress()
              onClose()
            }}
          >
            Cancel
          </button>
        </div>
      </div>
    </Modal>
  )
}
