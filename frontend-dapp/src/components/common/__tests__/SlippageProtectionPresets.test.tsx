import { describe, expect, it, vi } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SlippageProtectionPresets } from '@/components/common/SlippageProtectionPresets'
import { DEFAULT_SLIPPAGE_TOLERANCE_PERCENT, SLIPPAGE_PROTECTION_LABEL } from '@/utils/slippageProtectionCopy'
import { TRADE_SLIPPAGE_PRESET_CLASS } from '@/utils/tradeMoneyCta'

describe('SlippageProtectionPresets (GitLab #528)', () => {
  it('keeps the label outside the chip group and defaults 5% active', () => {
    render(
      <SlippageProtectionPresets
        selectedPercent={DEFAULT_SLIPPAGE_TOLERANCE_PERCENT}
        onSelect={() => {}}
        chipClassName={TRADE_SLIPPAGE_PRESET_CLASS}
        groupTestId="trade-market-slippage-presets"
        presetTestIdPrefix="trade-market-slippage-preset-"
        showColon
      />
    )

    const group = screen.getByTestId('trade-market-slippage-presets')
    const label = screen.getByTestId('trade-market-slippage-presets-label')
    expect(label).toHaveTextContent(`${SLIPPAGE_PROTECTION_LABEL}:`)
    expect(label).not.toHaveAttribute('tabindex')
    expect(group).toHaveAttribute('role', 'group')
    expect(group).toHaveAttribute('aria-labelledby', label.id)
    expect(group.className).toMatch(/grid-cols-3/)
    expect(within(group).queryByText(SLIPPAGE_PROTECTION_LABEL)).not.toBeInTheDocument()

    expect(screen.getByTestId('trade-market-slippage-preset-0.5')).toHaveClass('tab-glass-inactive')
    expect(screen.getByTestId('trade-market-slippage-preset-1')).toHaveClass('tab-glass-inactive')
    expect(screen.getByTestId('trade-market-slippage-preset-5')).toHaveClass('tab-glass-active')
    expect(screen.getByTestId('trade-market-slippage-preset-5')).toHaveAttribute('aria-pressed', 'true')
  })

  it('selects only the clicked chip and does not mount a second group', async () => {
    const user = userEvent.setup()
    const onSelect = vi.fn()
    const { rerender } = render(
      <SlippageProtectionPresets
        selectedPercent={5}
        onSelect={onSelect}
        chipClassName={TRADE_SLIPPAGE_PRESET_CLASS}
        groupTestId="trade-market-slippage-presets"
        presetTestIdPrefix="trade-market-slippage-preset-"
      />
    )

    await user.click(screen.getByTestId('trade-market-slippage-preset-0.5'))
    expect(onSelect).toHaveBeenCalledTimes(1)
    expect(onSelect).toHaveBeenCalledWith(0.5)

    rerender(
      <SlippageProtectionPresets
        selectedPercent={0.5}
        onSelect={onSelect}
        chipClassName={TRADE_SLIPPAGE_PRESET_CLASS}
        groupTestId="trade-market-slippage-presets"
        presetTestIdPrefix="trade-market-slippage-preset-"
      />
    )
    expect(screen.getByTestId('trade-market-slippage-preset-0.5')).toHaveClass('tab-glass-active')
    expect(screen.getByTestId('trade-market-slippage-preset-1')).toHaveClass('tab-glass-inactive')
    expect(screen.getByTestId('trade-market-slippage-preset-5')).toHaveClass('tab-glass-inactive')
    expect(screen.getAllByTestId('trade-market-slippage-presets')).toHaveLength(1)
  })

  it('renders Custom outside the chip group so it cannot sit between presets', () => {
    render(
      <SlippageProtectionPresets
        selectedPercent={5}
        onSelect={() => {}}
        customActive
        chipClassName="tab-glass !text-xs !px-3 !py-1.5"
        groupTestId="swap-slippage-presets"
        presetTestIdPrefix="swap-slippage-preset-"
        customSlot={<input data-testid="swap-slippage-custom" aria-label="Custom slippage protection (percent)" />}
      />
    )

    const group = screen.getByTestId('swap-slippage-presets')
    expect(within(group).queryByTestId('swap-slippage-custom')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-slippage-custom')).toBeInTheDocument()
    expect(screen.getByTestId('swap-slippage-preset-5')).toHaveClass('tab-glass-inactive')
  })

  it('keeps Trade chips at the #417 min-h-11 touch target', () => {
    render(
      <SlippageProtectionPresets
        selectedPercent={5}
        onSelect={() => {}}
        chipClassName={TRADE_SLIPPAGE_PRESET_CLASS}
        groupTestId="trade-market-slippage-presets"
        presetTestIdPrefix="trade-market-slippage-preset-"
      />
    )
    expect(TRADE_SLIPPAGE_PRESET_CLASS).toMatch(/min-h-11/)
    expect(screen.getByTestId('trade-market-slippage-preset-0.5').className).toMatch(/min-h-11/)
  })
})
