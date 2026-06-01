import { clampLimitOrderMaxAdjustSteps, LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT } from '@/utils/limitOrderExpiry'

/** Minimum adaptive steps — absorbs normal book churn between quote and submit (GitLab #268). */
export const LIMIT_LADDER_ADAPTIVE_STEPS_FLOOR = LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT

/** On-chain hard cap (`MAX_ADJUST_STEPS_HARD_CAP` in dex-common). */
export const LIMIT_LADDER_ADAPTIVE_STEPS_HARD_CAP = 256

/** Multiplier applied to observed local depth when sizing the step budget. */
export const LIMIT_LADDER_ADAPTIVE_DEPTH_MULTIPLIER = 2

/** Books with at most this many orders in the ladder band use the cheap ladder path. */
export const LIMIT_LADDER_THIN_BOOK_ORDER_THRESHOLD = 3

export interface AdaptiveMaxAdjustStepsInput {
  /** Orders in the probed price window (excluding ladder rungs). */
  localDepth: number
  /** Head → boundary rung distance in orders (0 when empty / head insert). */
  headToBoundaryDistance: number
  rungCount: number
}

/**
 * Derive a conservative `max_adjust_steps` from probed depth (GitLab #268).
 * `clamp(observed × multiplier + floor_component, FLOOR, HARD_CAP)`.
 */
export function computeAdaptiveMaxAdjustSteps(input: AdaptiveMaxAdjustStepsInput): number {
  const observed = Math.max(input.localDepth, input.headToBoundaryDistance, input.rungCount - 1)
  const raw = Math.ceil(observed * LIMIT_LADDER_ADAPTIVE_DEPTH_MULTIPLIER + LIMIT_LADDER_ADAPTIVE_STEPS_FLOOR / 2)
  const clamped = Math.max(LIMIT_LADDER_ADAPTIVE_STEPS_FLOOR, Math.min(LIMIT_LADDER_ADAPTIVE_STEPS_HARD_CAP, raw))
  return clampLimitOrderMaxAdjustSteps(clamped)
}
