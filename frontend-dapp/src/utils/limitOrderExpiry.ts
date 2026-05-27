/**
 * Expiry for `PlaceLimitOrder` is `expires_at` in Unix **seconds** (see `docs/limit-orders.md`).
 * Helpers convert between local `datetime-local` values and on-chain seconds.
 */

/** Sensible default for the book-head insert walk; pair hard cap is higher. */
export const LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT = 32
/** UI clamp; must not exceed the pair’s hard cap in `pair.rs` / `docs/limit-orders.md`. */
export const LIMIT_ORDER_MAX_ADJUST_STEPS_MAX_UI = 256

/**
 * Retail gas presets for `PlaceLimitOrder.max_adjust_steps` (GitLab #204).
 * Higher step cap → more placement gas, better reliability on deep books.
 */
export const LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES = {
  low: 16,
  medium: LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT,
  high: 128,
} as const

export type LimitOrderMaxAdjustStepsPresetTier = keyof typeof LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES | 'custom'

export const LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_TIERS: readonly Exclude<
  LimitOrderMaxAdjustStepsPresetTier,
  'custom'
>[] = ['low', 'medium', 'high']

/** Map on-chain step integer → active retail preset tier (non-preset values → `custom`). */
export function resolveLimitOrderMaxAdjustStepsPresetTier(maxSteps: number): LimitOrderMaxAdjustStepsPresetTier {
  if (maxSteps === LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES.low) return 'low'
  if (maxSteps === LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES.medium) return 'medium'
  if (maxSteps === LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES.high) return 'high'
  return 'custom'
}

/** On-chain `max_adjust_steps` for Low / Medium / High preset buttons. */
export function limitOrderMaxAdjustStepsForPresetTier(
  tier: Exclude<LimitOrderMaxAdjustStepsPresetTier, 'custom'>
): number {
  return LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES[tier]
}

/** Clamp user-entered custom steps to the pair UI range `1…256`. */
export function clampLimitOrderMaxAdjustSteps(raw: number): number {
  if (!Number.isFinite(raw)) return LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT
  const n = Math.floor(raw)
  if (n < 1) return 1
  if (n > LIMIT_ORDER_MAX_ADJUST_STEPS_MAX_UI) return LIMIT_ORDER_MAX_ADJUST_STEPS_MAX_UI
  return n
}

const SEC_PER_DAY = 86_400

/** Preset from "now" (ms) for 24h / 7d — same as typing that future Unix second into a legacy text field. */
export function limitOrderExpiryFromPreset24h(nowMs: number): number {
  return Math.floor(nowMs / 1000) + SEC_PER_DAY
}

export function limitOrderExpiryFromPreset7d(nowMs: number): number {
  return Math.floor(nowMs / 1000) + 7 * SEC_PER_DAY
}

/**
 * Parse `datetime-local` string (local wall time, no zone suffix) to Unix seconds.
 * Empty or invalid → null.
 */
export function localDatetimeInputToUnixSeconds(value: string): number | null {
  const t = value.trim()
  if (!t) return null
  const ms = new Date(t).getTime()
  if (!Number.isFinite(ms)) return null
  return Math.floor(ms / 1000)
}

/** Value for `<input type="datetime-local" />` from Unix seconds, or '' when null. */
export function unixSecondsToLocalDatetimeInputValue(sec: number | null): string {
  if (sec == null || !Number.isFinite(sec)) return ''
  const d = new Date(sec * 1000)
  if (!Number.isFinite(d.getTime())) return ''
  const pad = (n: number) => String(n).padStart(2, '0')
  const y = d.getFullYear()
  const mon = pad(d.getMonth() + 1)
  const day = pad(d.getDate())
  const h = pad(d.getHours())
  const min = pad(d.getMinutes())
  return `${y}-${mon}-${day}T${h}:${min}`
}

/** Parse optional raw positive integer string; empty → null, invalid → NaN signal via null from caller. */
export function parseRawExpiresUnixInput(raw: string): number | 'invalid' | null {
  const t = raw.trim()
  if (!t) return null
  if (!/^\d+$/.test(t)) return 'invalid'
  const n = Number(t)
  if (!Number.isSafeInteger(n) || n < 0) return 'invalid'
  return n
}
