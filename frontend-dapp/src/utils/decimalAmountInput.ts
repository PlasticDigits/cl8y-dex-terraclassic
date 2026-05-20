/**
 * Human amount draft validation for controlled inputs (Swap "You Pay", hybrid book leg, etc.).
 * Locale commas are rejected at the field — only `.` as decimal separator ([GitLab #169](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169)).
 */
export const DECIMAL_AMOUNT_DRAFT_RE = /^\d*\.?\d*$/

/** True when `value` is empty or contains only digits with at most one `.` (no leading/trailing junk). */
export function isDecimalAmountDraft(value: string): boolean {
  return DECIMAL_AMOUNT_DRAFT_RE.test(value)
}

/** Parse a raw integer string for chain math; returns `null` on invalid input instead of throwing. */
export function tryParseBigInt(raw: string): bigint | null {
  const s = raw.trim()
  if (!s || !/^\d+$/.test(s)) return null
  try {
    return BigInt(s)
  } catch {
    return null
  }
}
