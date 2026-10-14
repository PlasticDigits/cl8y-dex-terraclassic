import { useState } from 'react'
import { Modal } from '@/components/ui'
import { EXPERT_MODE_CONFIRM_PHRASE } from '@/utils/expertMode'
import { sounds } from '@/lib/sounds'

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
          Expert Mode permits fair-reference deviation above <strong style={{ color: 'var(--ink)' }}>30%</strong>; you
          may receive much less than expected or a suspiciously favorable quote.
        </p>
        <p>
          Your selected price protection and receive minimums still apply; fair-reference deviation of 99% or more
          remains blocked.
        </p>
        <p className="font-semibold" style={{ color: 'var(--color-warning, #f59e0b)' }}>
          Only enable this mode if you understand the risk.
        </p>
        <div className="space-y-2">
          <label htmlFor="expert-mode-confirm" className="block text-xs font-medium uppercase tracking-wide">
            Type <span className="font-mono normal-case">{EXPERT_MODE_CONFIRM_PHRASE}</span> to confirm
          </label>
          <input
            id="expert-mode-confirm"
            type="text"
            className="input-glass w-full"
            value={typedPhrase}
            onChange={(e) => setTypedPhrase(e.target.value)}
            autoComplete="off"
            spellCheck={false}
            data-testid="expert-mode-confirm-input"
            aria-label="Expert mode confirmation phrase"
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
