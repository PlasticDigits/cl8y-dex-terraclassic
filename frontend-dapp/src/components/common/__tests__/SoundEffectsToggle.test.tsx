import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SoundEffectsToggle } from '@/components/common/SoundEffectsToggle'

describe('SoundEffectsToggle', () => {
  it('exposes aria-pressed for enabled state and toggles via click', async () => {
    const user = userEvent.setup()
    const onToggle = vi.fn()

    const { rerender } = render(<SoundEffectsToggle enabled onToggle={onToggle} labelStyle="short" />)

    const button = screen.getByRole('button', { name: 'Mute sound effects' })
    expect(button).toHaveAttribute('aria-pressed', 'true')
    expect(button).toHaveTextContent('Sound')

    await user.click(button)
    expect(onToggle).toHaveBeenCalledWith(false)

    rerender(<SoundEffectsToggle enabled={false} onToggle={onToggle} labelStyle="short" />)

    const muted = screen.getByRole('button', { name: 'Enable sound effects' })
    expect(muted).toHaveAttribute('aria-pressed', 'false')
    expect(muted).toHaveTextContent('Muted')
  })

  it('uses long labels for mobile style', () => {
    render(<SoundEffectsToggle enabled onToggle={() => {}} labelStyle="long" />)
    expect(screen.getByRole('button', { name: 'Mute sound effects' })).toHaveTextContent('Sound on')
  })
})
