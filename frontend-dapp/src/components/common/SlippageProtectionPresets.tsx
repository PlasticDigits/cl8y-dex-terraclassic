import { useId, type CSSProperties, type ReactNode } from 'react'
import { SLIPPAGE_PROTECTION_LABEL, SLIPPAGE_TOLERANCE_PRESETS_PERCENT } from '@/utils/slippageProtectionCopy'

export type SlippageProtectionPresetsProps = {
  selectedPercent: number
  onSelect: (percent: number) => void
  /** When true, no preset is treated as selected (Swap Custom is active). */
  customActive?: boolean
  chipClassName: string
  groupTestId: string
  presetTestIdPrefix: string
  labelClassName?: string
  labelStyle?: CSSProperties
  showColon?: boolean
  /** Swap Custom (or other sibling) — rendered outside the chip `role="group"`. */
  customSlot?: ReactNode
}

/**
 * Retail 0.5 / 1 / 5% Slippage protection chips as one aligned group (GitLab #528).
 * Label sits above the group; Custom (if any) stacks below the group, never a wrap
 * sibling next to 0.5%. Chips use a 3-up grid so they shrink together on a narrow ticket.
 */
export function SlippageProtectionPresets({
  selectedPercent,
  onSelect,
  customActive = false,
  chipClassName,
  groupTestId,
  presetTestIdPrefix,
  labelClassName,
  labelStyle,
  showColon = false,
  customSlot,
}: SlippageProtectionPresetsProps) {
  const uid = useId()
  const labelId = `${uid}-slippage-protection-label`

  return (
    <div className="space-y-2">
      <p id={labelId} className={labelClassName} style={labelStyle} data-testid={`${groupTestId}-label`}>
        {SLIPPAGE_PROTECTION_LABEL}
        {showColon ? ':' : ''}
      </p>
      <div className={customSlot ? 'flex flex-col gap-2' : undefined}>
        <div
          role="group"
          aria-labelledby={labelId}
          data-testid={groupTestId}
          className="grid min-w-0 w-full grid-cols-3 items-center gap-2"
        >
          {SLIPPAGE_TOLERANCE_PRESETS_PERCENT.map((v) => {
            const active = !customActive && selectedPercent === v
            return (
              <button
                key={v}
                type="button"
                className={`${chipClassName} ${active ? 'tab-glass-active' : 'tab-glass-inactive'}`}
                data-testid={`${presetTestIdPrefix}${v}`}
                aria-pressed={active}
                onClick={() => onSelect(v)}
              >
                {v}%
              </button>
            )
          })}
        </div>
        {customSlot}
      </div>
    </div>
  )
}
