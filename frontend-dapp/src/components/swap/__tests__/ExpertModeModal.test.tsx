import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ExpertModeModal } from '@/components/swap/ExpertModeModal'
import { EXPERT_MODE_CONFIRM_PHRASE } from '@/utils/expertMode'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

describe('ExpertModeModal', () => {
  it('requires typing the confirmation phrase before enabling (GitLab #378)', async () => {
    const user = userEvent.setup()
    const onEnable = vi.fn()
    render(<ExpertModeModal isOpen onClose={() => {}} onEnable={onEnable} />)

    const enableBtn = screen.getByTestId('expert-mode-confirm-enable')
    expect(enableBtn).toBeDisabled()

    await user.type(screen.getByTestId('expert-mode-confirm-input'), EXPERT_MODE_CONFIRM_PHRASE)
    expect(enableBtn).toBeEnabled()

    await user.click(enableBtn)
    expect(onEnable).toHaveBeenCalledTimes(1)
  })

  it('does not enable when the phrase is wrong', async () => {
    const user = userEvent.setup()
    const onEnable = vi.fn()
    render(<ExpertModeModal isOpen onClose={() => {}} onEnable={onEnable} />)

    await user.type(screen.getByTestId('expert-mode-confirm-input'), 'enable expert mode')
    const enableBtn = screen.getByTestId('expert-mode-confirm-enable')
    expect(enableBtn).toBeDisabled()
    await user.click(enableBtn)
    expect(onEnable).not.toHaveBeenCalled()
  })
})
