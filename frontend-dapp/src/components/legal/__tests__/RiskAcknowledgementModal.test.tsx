import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import RiskAcknowledgementModal from '@/components/legal/RiskAcknowledgementModal'
import { RISK_ACK_STORAGE_KEY } from '@/utils/riskAcknowledgement'

vi.mock('@/utils/riskAcknowledgement', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/riskAcknowledgement')>()
  return {
    ...actual,
    skipRiskAcknowledgementForAutomation: () => false,
  }
})

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

describe('RiskAcknowledgementModal', () => {
  beforeEach(() => {
    localStorage.removeItem(RISK_ACK_STORAGE_KEY)
  })

  it('requires checkbox before continue', async () => {
    render(<RiskAcknowledgementModal />)
    expect(screen.getByRole('dialog', { name: /risk acknowledgement/i })).toBeVisible()

    const proceed = screen.getByRole('button', { name: /continue to the app/i })
    expect(proceed).toBeDisabled()

    fireEvent.click(screen.getByRole('checkbox'))
    expect(proceed).not.toBeDisabled()

    fireEvent.click(proceed)
    expect(localStorage.getItem(RISK_ACK_STORAGE_KEY)).toBeTruthy()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('has no close control, backdrop, or Escape dismiss (GitLab #138 / #672 D7)', () => {
    render(<RiskAcknowledgementModal />)
    expect(screen.getByRole('dialog', { name: /risk acknowledgement/i })).toBeVisible()
    expect(screen.queryByTestId('modal-close')).not.toBeInTheDocument()
    fireEvent.click(screen.getByTestId('modal-backdrop'))
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(screen.getByRole('dialog', { name: /risk acknowledgement/i })).toBeVisible()
    expect(localStorage.getItem(RISK_ACK_STORAGE_KEY)).toBeNull()
  })
})
