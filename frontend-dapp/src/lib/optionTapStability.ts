/** Ignore a click if IME / chrome shifted the option under the finger (GitLab #632). */
export const OPTION_TAP_MOVE_THRESHOLD_PX = 12

export function optionMovedBeyondThreshold(
  start: DOMRectReadOnly,
  now: DOMRectReadOnly,
  threshold = OPTION_TAP_MOVE_THRESHOLD_PX
): boolean {
  return Math.abs(now.top - start.top) > threshold || Math.abs(now.left - start.left) > threshold
}
