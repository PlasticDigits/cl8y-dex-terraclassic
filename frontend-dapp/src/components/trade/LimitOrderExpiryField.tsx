import { useState, useEffect } from 'react'
import {
  limitOrderExpiryFromPreset24h,
  limitOrderExpiryFromPreset7d,
  localDatetimeInputToUnixSeconds,
  parseRawExpiresUnixInput,
  unixSecondsToLocalDatetimeInputValue,
} from '@/utils/limitOrderExpiry'
import { DOCS_GITLAB_BASE } from '@/utils/constants'

const LIMIT_ORDERS_DOC = `${DOCS_GITLAB_BASE}/limit-orders.md`

type Props = {
  /** `expires_at` for the pair hook: Unix seconds, or `null` for no expiry. */
  value: number | null
  onChange: (next: number | null) => void
  idPrefix: string
  /** Smaller type scale (trade ticket panel). */
  compact?: boolean
  /** Injected clock for tests (defaults to `Date.now`). */
  nowMs?: () => number
}

/**
 * Local date/time + presets; still submits Unix seconds to `placeLimitOrder` (see `docs/limit-orders.md`).
 * Raw-seconds override lives under Advanced in {@link LimitOrderAdvancedLimitSettings}.
 */
export function LimitOrderExpiryField({ value, onChange, idPrefix, compact, nowMs = () => Date.now() }: Props) {
  const inputClass = (compact ? 'input-neo w-full text-sm' : 'input-neo w-full') + ' font-sans'
  const btnClass = compact
    ? 'px-2 py-1 rounded-md text-[10px] uppercase tracking-wide border border-white/10 hover:bg-white/5'
    : 'px-2.5 py-1.5 rounded-lg text-xs uppercase tracking-wide border border-white/10 hover:bg-white/5'
  const hintClass = compact ? 'text-[10px] leading-snug' : 'text-xs'

  return (
    <div className="space-y-2">
      <div className="flex flex-col gap-1">
        <label className="label-neo" htmlFor={`${idPrefix}-expiry-dt`}>
          Expires
        </label>
        <p className={hintClass} style={{ color: 'var(--ink-dim)' }}>
          Choose a local date and time, a quick preset, or set Advanced → raw seconds. On-chain:{' '}
          <code className="text-[0.85em]">expires_at</code> in the hook message.{' '}
          <a className="underline hover:opacity-80" href={LIMIT_ORDERS_DOC} target="_blank" rel="noopener noreferrer">
            Expiry rules
          </a>
        </p>
      </div>
      <div className="flex flex-wrap gap-2">
        <button type="button" className={btnClass} onClick={() => onChange(null)} style={{ color: 'var(--ink-dim)' }}>
          No expiry
        </button>
        <button type="button" className={btnClass} onClick={() => onChange(limitOrderExpiryFromPreset24h(nowMs()))}>
          24h
        </button>
        <button type="button" className={btnClass} onClick={() => onChange(limitOrderExpiryFromPreset7d(nowMs()))}>
          7d
        </button>
      </div>
      <input
        id={`${idPrefix}-expiry-dt`}
        type="datetime-local"
        className={inputClass}
        value={unixSecondsToLocalDatetimeInputValue(value)}
        onChange={(e) => {
          const t = e.target.value
          if (!t) {
            onChange(null)
            return
          }
          const sec = localDatetimeInputToUnixSeconds(t)
          onChange(sec)
        }}
      />
    </div>
  )
}

type RawProps = {
  value: number | null
  onChange: (next: number | null) => void
  idPrefix: string
  compact?: boolean
}

/**
 * Power-user line under Advanced: optional on-chain `expires_at` in Unix seconds.
 * Empty clears expiry; must agree with the friendly controls when both are used.
 */
export function LimitOrderExpiresAtRawField({ value, onChange, idPrefix, compact }: RawProps) {
  const [text, setText] = useState(() => (value != null ? String(value) : ''))
  useEffect(() => {
    setText(value != null ? String(value) : '')
  }, [value])
  return (
    <div>
      <label className={compact ? 'label-neo text-[10px]' : 'label-neo'} htmlFor={`${idPrefix}-raw-expiry`}>
        Raw expiry (Unix seconds, optional)
      </label>
      <input
        id={`${idPrefix}-raw-expiry`}
        type="text"
        inputMode="numeric"
        className={compact ? 'input-neo w-full font-mono text-sm' : 'input-neo w-full font-mono'}
        placeholder="Leave empty for no expiry"
        value={text}
        onChange={(e) => {
          const t = e.target.value
          setText(t)
          const p = parseRawExpiresUnixInput(t)
          if (p === 'invalid') return
          onChange(p)
        }}
      />
    </div>
  )
}
