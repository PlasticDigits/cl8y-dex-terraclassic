import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ThemeSegmentedControl } from '@/components/common/ThemeSegmentedControl'

describe('ThemeSegmentedControl', () => {
  it('calls onSelect with the chosen mode and exposes aria-pressed', async () => {
    const user = userEvent.setup()
    const onSelect = vi.fn()

    const { rerender } = render(
      <ThemeSegmentedControl
        theme="dark"
        onSelect={onSelect}
        groupClassName="app-footer-theme-group"
        labelStyle="short"
      />
    )

    await user.click(screen.getByRole('button', { name: 'Light' }))
    expect(onSelect).toHaveBeenCalledWith('light')

    rerender(
      <ThemeSegmentedControl
        theme="light"
        onSelect={onSelect}
        groupClassName="app-footer-theme-group"
        labelStyle="short"
      />
    )

    expect(screen.getByRole('button', { name: 'Dark' })).toHaveAttribute('aria-pressed', 'false')
    expect(screen.getByRole('button', { name: 'Light' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('uses long labels for mobile style', () => {
    render(
      <ThemeSegmentedControl
        theme="dark"
        onSelect={() => {}}
        groupClassName="app-mobile-theme-group"
        labelStyle="long"
      />
    )

    expect(screen.getByRole('button', { name: 'Dark theme' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Light theme' })).toBeInTheDocument()
  })
})
