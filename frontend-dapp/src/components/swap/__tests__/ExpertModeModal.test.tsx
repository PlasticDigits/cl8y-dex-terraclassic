import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ExpertModeModal } from '@/components/swap/ExpertModeModal'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

describe('ExpertModeModal', () => {
  it('requires typed confirmation before enabling expert mode (GitLab #378)', async () => {
    const user = userEvent.setup()
    const onEnable = vi.fn()
    render(<ExpertModeModal isOpen onClose={vi.fn()} onEnable={onEnable} />)

    const enableButton = screen.getByTestId('expert-mode-enable-button')
    expect(enableButton).toBeDisabled()

    await user.click(enableButton)
    expect(onEnable).not.toHaveBeenCalled()

    await user.type(screen.getByTestId('expert-mode-confirm-input'), 'enable expert mode')
    expect(enableButton).toBeEnabled()

    await user.click(enableButton)
    expect(onEnable).toHaveBeenCalledTimes(1)
  })
})
