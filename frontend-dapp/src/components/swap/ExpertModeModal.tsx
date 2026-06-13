import { useState } from 'react'
import { Modal } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { EXPERT_MODE_CONFIRM_PHRASE } from '@/utils/expertMode'

export interface ExpertModeModalProps {
  isOpen: boolean
  onClose: () => void
  onEnable: () => void
}

export function ExpertModeModal({ isOpen, onClose, onEnable }: ExpertModeModalProps) {
  const [typedPhrase, setTypedPhrase] = useState('')

  const phraseMatches = typedPhrase.trim() === EXPERT_MODE_CONFIRM_PHRASE

  const handleClose = () => {
    setTypedPhrase('')
    onClose()
  }

  const handleEnable = () => {
    if (!phraseMatches) return
    sounds.playButtonPress()
    setTypedPhrase('')
    onEnable()
  }

  return (
    <Modal isOpen={isOpen} onClose={handleClose} title="Enable Expert Mode" panelClassName="max-w-md">
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
        <div className="space-y-2">
          <label htmlFor="expert-mode-confirm" className="block text-xs font-medium uppercase tracking-wide">
            Type <span className="font-mono normal-case">{EXPERT_MODE_CONFIRM_PHRASE}</span> to confirm
          </label>
          <input
            id="expert-mode-confirm"
            type="text"
            className="input-neo w-full"
            value={typedPhrase}
            onChange={(e) => setTypedPhrase(e.target.value)}
            autoComplete="off"
            spellCheck={false}
            data-testid="expert-mode-confirm-input"
          />
        </div>
        <div className="flex flex-col sm:flex-row gap-2 pt-2">
          <button
            type="button"
            className="btn-primary flex-1"
            disabled={!phraseMatches}
            onClick={handleEnable}
            data-testid="expert-mode-confirm-enable"
          >
            Enable Expert Mode
          </button>
          <button type="button" className="btn-muted flex-1" onClick={handleClose}>
            Cancel
          </button>
        </div>
      </div>
    </Modal>
  )
}
