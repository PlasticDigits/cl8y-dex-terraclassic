import { useEffect, useState, type ReactNode } from 'react'
import {
  LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_TIERS,
  clampLimitOrderMaxAdjustSteps,
  limitOrderMaxAdjustStepsForPresetTier,
  resolveLimitOrderMaxAdjustStepsPresetTier,
  type LimitOrderMaxAdjustStepsPresetTier,
} from '@/utils/limitOrderExpiry'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { LimitOrderExpiresAtRawField } from './LimitOrderExpiryField'

const PLACEMENT_GAS_DOC = `${DOCS_GITLAB_BASE}/limit-orders.md#messages-cosmwasm`

const PRESET_LABELS: Record<Exclude<LimitOrderMaxAdjustStepsPresetTier, 'custom'>, string> = {
  low: 'Low',
  medium: 'Medium',
  high: 'High',
}

const PRESET_HINTS: Record<Exclude<LimitOrderMaxAdjustStepsPresetTier, 'custom'>, string> = {
  low: 'Low gas',
  medium: 'Default',
  high: 'High gas',
}

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  maxSteps: number
  onMaxStepsChange: (n: number) => void
  expiresAt: number | null
  onExpiresAtChange: (n: number | null) => void
  idPrefix: string
  compact?: boolean
  /** Extra Advanced body (e.g. `/trade` expiry + price chips + pre-submit, #693). */
  children?: ReactNode
}

/**
 * Advanced limit settings: placement gas preset (book walk) and optional raw `expires_at` (seconds).
 */
export function LimitOrderAdvancedLimitSettings({
  open,
  onOpenChange,
  maxSteps,
  onMaxStepsChange,
  expiresAt,
  onExpiresAtChange,
  idPrefix,
  compact,
  children,
}: Props) {
  const sm = compact ? 'text-[10px] leading-snug' : 'text-xs'
  const presetBtn = 'px-1.5 py-0.5 rounded border border-white/10 text-[10px] uppercase tracking-wide hover:bg-white/5'
  const derivedTier = resolveLimitOrderMaxAdjustStepsPresetTier(maxSteps)
  const [customForced, setCustomForced] = useState(false)
  const activeTier = customForced ? 'custom' : derivedTier
  const customMode = activeTier === 'custom'

  useEffect(() => {
    if (derivedTier !== 'custom') setCustomForced(false)
  }, [derivedTier])

  const onPresetClick = (tier: Exclude<LimitOrderMaxAdjustStepsPresetTier, 'custom'>) => {
    setCustomForced(false)
    onMaxStepsChange(limitOrderMaxAdjustStepsForPresetTier(tier))
  }

  const onCustomClick = () => {
    setCustomForced(true)
  }

  return (
    <details open={open} onToggle={(e) => onOpenChange((e.currentTarget as HTMLDetailsElement).open)}>
      <summary
        className={
          compact
            ? 'cursor-pointer text-[10px] font-semibold uppercase tracking-wide'
            : 'cursor-pointer text-sm font-semibold uppercase tracking-wide'
        }
        style={{ color: 'var(--cyan)' }}
      >
        Advanced
      </summary>
      <div className="mt-3 space-y-3 pl-0 border-t border-white/10 pt-3">
        <div>
          <label className={compact ? 'label-glass text-[10px]' : 'label-glass'} htmlFor={`${idPrefix}-max-steps`}>
            Placement gas{' '}
            <a
              className="normal-case tracking-normal font-medium underline hover:opacity-80"
              href={PLACEMENT_GAS_DOC}
              target="_blank"
              rel="noopener noreferrer"
            >
              Docs
            </a>
          </label>
          <div className="flex flex-wrap gap-1.5 mt-2" role="group" aria-label="Placement gas preset">
            {LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_TIERS.map((tier) => (
              <button
                key={tier}
                type="button"
                className={presetBtn}
                style={{ color: activeTier === tier ? 'var(--cyan)' : 'var(--ink-dim)' }}
                title={PRESET_HINTS[tier]}
                aria-pressed={activeTier === tier}
                data-active={activeTier === tier ? 'true' : 'false'}
                onClick={() => onPresetClick(tier)}
              >
                {PRESET_LABELS[tier]}
              </button>
            ))}
            <button
              type="button"
              className={presetBtn}
              style={{ color: customMode ? 'var(--cyan)' : 'var(--ink-dim)' }}
              title="Set the on-chain step cap yourself. Higher = more gas."
              aria-pressed={customMode}
              data-active={customMode ? 'true' : 'false'}
              onClick={onCustomClick}
            >
              Custom
            </button>
          </div>
          {customMode ? (
            <input
              id={`${idPrefix}-max-steps`}
              type="number"
              className={compact ? 'input-glass w-full text-sm mt-2' : 'input-glass w-full mt-2'}
              min={1}
              max={256}
              value={maxSteps}
              onChange={(e) => onMaxStepsChange(clampLimitOrderMaxAdjustSteps(Number(e.target.value)))}
              aria-label="Custom placement gas steps (1–256)"
            />
          ) : (
            <p className={sm + ' mt-2'} style={{ color: 'var(--ink-dim)' }} data-testid={`${idPrefix}-preset-hint`}>
              {PRESET_HINTS[activeTier]}
            </p>
          )}
        </div>
        <LimitOrderExpiresAtRawField
          value={expiresAt}
          onChange={onExpiresAtChange}
          idPrefix={`${idPrefix}-adv`}
          compact={compact}
        />
        {children}
      </div>
    </details>
  )
}
