import { useCallback, useState } from 'react'
import { Modal } from '@/components/ui/Modal'
import { ENVIRONMENT_EXPLAINER, NFA_SHORT, RISK_SUMMARY_BULLETS } from '@/components/legal/legalCopy'
import {
  hasRiskAcknowledgement,
  setRiskAcknowledged,
  skipRiskAcknowledgementForAutomation,
} from '@/utils/riskAcknowledgement'
import { sounds } from '@/lib/sounds'

const noop = () => {}

export default function RiskAcknowledgementModal() {
  const [open, setOpen] = useState(
    () => !skipRiskAcknowledgementForAutomation() && typeof window !== 'undefined' && !hasRiskAcknowledgement()
  )
  const [readChecked, setReadChecked] = useState(false)

  const acknowledge = useCallback(() => {
    sounds.playButtonPress()
    setRiskAcknowledged()
    setOpen(false)
  }, [])

  if (!open) return null

  return (
    <Modal
      isOpen
      dismissible={false}
      onClose={noop}
      title="Risk acknowledgement"
      panelClassName="app-modal-panel--risk"
    >
      <div className="app-risk-modal-body">
        <p className="app-risk-modal-lead">{NFA_SHORT}</p>
        <ul className="app-risk-modal-list">
          {RISK_SUMMARY_BULLETS.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
        <p className="app-risk-modal-meta">{ENVIRONMENT_EXPLAINER}</p>
        <label className="app-risk-modal-confirm">
          <input
            type="checkbox"
            checked={readChecked}
            onChange={(e) => {
              sounds.playButtonPress()
              setReadChecked(e.target.checked)
            }}
          />
          <span>I understand these risks and that this application is provided as-is without warranty.</span>
        </label>
        <button type="button" className="btn-primary btn-cta w-full mt-4" disabled={!readChecked} onClick={acknowledge}>
          Continue to the app
        </button>
      </div>
    </Modal>
  )
}
