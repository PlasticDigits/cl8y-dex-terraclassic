import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ExpertModeModal } from './ExpertModeModal'
import { EXPERT_MODE_CONFIRM_PHRASE } from '@/utils/expertMode'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

describe('ExpertModeModal (GitLab #378)', () => {
  it('requires typing the confirmation phrase before enabling', async () => {
    const user = userEvent.setup()
    const onEnable = vi.fn()
    const onClose = vi.fn()

    render(<ExpertModeModal isOpen onClose={onClose} onEnable={onEnable} />)

    const enableBtn = screen.getByTestId('expert-mode-confirm-enable')
    expect(enableBtn).toBeDisabled()

    await user.type(screen.getByTestId('expert-mode-confirm-input'), EXPERT_MODE_CONFIRM_PHRASE)
    expect(enableBtn).toBeEnabled()

    await user.click(enableBtn)
    expect(onEnable).toHaveBeenCalledTimes(1)
    expect(onClose).not.toHaveBeenCalled()
  })

  it('does not enable when phrase is wrong', async () => {
    const user = userEvent.setup()
    const onEnable = vi.fn()

    render(<ExpertModeModal isOpen onClose={() => {}} onEnable={onEnable} />)

    await user.type(screen.getByTestId('expert-mode-confirm-input'), 'wrong phrase')
    await user.click(screen.getByTestId('expert-mode-confirm-enable'))
    expect(onEnable).not.toHaveBeenCalled()
  })
})
