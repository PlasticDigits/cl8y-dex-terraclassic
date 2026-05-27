import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { LimitOrderAdvancedLimitSettings } from '../LimitOrderAdvancedLimitSettings'
import {
  LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT,
  LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES,
} from '@/utils/limitOrderExpiry'

describe('LimitOrderAdvancedLimitSettings (GitLab #204)', () => {
  const baseProps = {
    open: true,
    onOpenChange: vi.fn(),
    expiresAt: null as number | null,
    onExpiresAtChange: vi.fn(),
    idPrefix: 'test-adv',
  }

  it('shows Low / Medium / High / Custom presets (no raw step numbers as labels)', () => {
    render(
      <LimitOrderAdvancedLimitSettings
        {...baseProps}
        maxSteps={LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT}
        onMaxStepsChange={vi.fn()}
      />
    )
    expect(screen.getByRole('button', { name: 'Low' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Medium' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'High' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Custom' })).toBeInTheDocument()
    expect(screen.queryByText('max_adjust_steps')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '16' })).not.toBeInTheDocument()
  })

  it('marks Medium active when maxSteps is the retail default (32)', () => {
    render(
      <LimitOrderAdvancedLimitSettings
        {...baseProps}
        maxSteps={LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT}
        onMaxStepsChange={vi.fn()}
      />
    )
    const mediumBtn = screen.getByRole('button', { name: 'Medium' })
    expect(mediumBtn.getAttribute('aria-pressed')).toBe('true')
    expect(mediumBtn.getAttribute('data-active')).toBe('true')
    expect(screen.queryByLabelText(/max-steps/i)).not.toBeInTheDocument()
  })

  it('calls onMaxStepsChange with mapped preset integers', () => {
    const onMaxStepsChange = vi.fn()
    render(
      <LimitOrderAdvancedLimitSettings
        {...baseProps}
        maxSteps={LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT}
        onMaxStepsChange={onMaxStepsChange}
      />
    )
    fireEvent.click(screen.getByRole('button', { name: 'Low' }))
    expect(onMaxStepsChange).toHaveBeenCalledWith(LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES.low)
    fireEvent.click(screen.getByRole('button', { name: 'High' }))
    expect(onMaxStepsChange).toHaveBeenCalledWith(LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES.high)
  })

  it('reveals numeric input when Custom is selected and clamps on change', () => {
    const onMaxStepsChange = vi.fn()
    const { rerender } = render(
      <LimitOrderAdvancedLimitSettings
        {...baseProps}
        maxSteps={LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT}
        onMaxStepsChange={onMaxStepsChange}
      />
    )
    fireEvent.click(screen.getByRole('button', { name: 'Custom' }))
    expect(screen.getByRole('spinbutton')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Custom' }).getAttribute('aria-pressed')).toBe('true')

    rerender(<LimitOrderAdvancedLimitSettings {...baseProps} maxSteps={99} onMaxStepsChange={onMaxStepsChange} />)
    const input = screen.getByRole('spinbutton')
    fireEvent.change(input, { target: { value: '300' } })
    expect(onMaxStepsChange).toHaveBeenCalledWith(256)

    rerender(<LimitOrderAdvancedLimitSettings {...baseProps} maxSteps={99} onMaxStepsChange={onMaxStepsChange} />)
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '0' } })
    expect(onMaxStepsChange).toHaveBeenCalledWith(1)
  })

  it('shows gas tradeoff helper copy and human-readable docs link', () => {
    render(
      <LimitOrderAdvancedLimitSettings
        {...baseProps}
        maxSteps={LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT}
        onMaxStepsChange={vi.fn()}
      />
    )
    expect(screen.getByText(/how much gas the placement transaction may spend/i)).toBeInTheDocument()
    const docLink = screen.getByRole('link', { name: /how book placement gas works/i })
    expect(docLink.getAttribute('href')).toMatch(/limit-orders\.md#messages-cosmwasm/)
  })
})
