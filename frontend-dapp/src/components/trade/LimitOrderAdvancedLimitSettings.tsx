import { LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT, LIMIT_ORDER_MAX_ADJUST_STEPS_MAX_UI } from '@/utils/limitOrderExpiry'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { LimitOrderExpiresAtRawField } from './LimitOrderExpiryField'

const MAX_ADJUST_DOC = `${DOCS_GITLAB_BASE}/limit-orders.md#messages-cosmwasm`

type Props = {
  open: boolean
  onOpenChange: (open: boolean) => void
  maxSteps: number
  onMaxStepsChange: (n: number) => void
  expiresAt: number | null
  onExpiresAtChange: (n: number | null) => void
  idPrefix: string
  compact?: boolean
}

const STEP_PRESETS = [16, 32, 64, 128] as const

/**
 * “Max adjust steps” and optional raw `expires_at` (seconds), for users who need more than retail defaults.
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
}: Props) {
  const sm = compact ? 'text-[10px] leading-snug' : 'text-xs'
  const presetBtn = 'px-1.5 py-0.5 rounded border border-white/10 text-[10px] uppercase tracking-wide hover:bg-white/5'

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
          <label className={compact ? 'label-neo text-[10px]' : 'label-neo'} htmlFor={`${idPrefix}-max-steps`}>
            Book insert walk limit
          </label>
          <p className={sm + ' mt-0.5'} style={{ color: 'var(--ink-dim)' }}>
            Caps how many on-chain steps the pair uses when slotting this order from the book head. Default{' '}
            {LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT} is enough for most books. See{' '}
            <a className="underline hover:opacity-80" href={MAX_ADJUST_DOC} target="_blank" rel="noopener noreferrer">
              max_adjust_steps (on-chain + docs)
            </a>
            .
          </p>
          <div className="flex flex-wrap gap-1.5 mt-2">
            {STEP_PRESETS.map((n) => (
              <button
                key={n}
                type="button"
                className={presetBtn}
                style={{ color: maxSteps === n ? 'var(--cyan)' : 'var(--ink-dim)' }}
                onClick={() => onMaxStepsChange(n)}
              >
                {n}
              </button>
            ))}
          </div>
          <input
            id={`${idPrefix}-max-steps`}
            type="number"
            className={compact ? 'input-neo w-full text-sm mt-2' : 'input-neo w-full mt-2'}
            min={1}
            max={LIMIT_ORDER_MAX_ADJUST_STEPS_MAX_UI}
            value={maxSteps}
            onChange={(e) => onMaxStepsChange(Number(e.target.value) || LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT)}
          />
        </div>
        <LimitOrderExpiresAtRawField
          value={expiresAt}
          onChange={onExpiresAtChange}
          idPrefix={`${idPrefix}-adv`}
          compact={compact}
        />
      </div>
    </details>
  )
}
