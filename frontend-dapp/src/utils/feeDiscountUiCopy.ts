/**
 * Shared fee-discount UX copy for Swap and Pool (GitLab #476).
 *
 * Invariants:
 * - Discount requires **hold** of the configured `cl8y_token` CW20 **and** `Register` on `/tiers`.
 * - Holding alone (or a differently named/bridged asset) does not apply a discount.
 * - CTA must stay a same-origin relative `/tiers` link (no external phishing targets).
 * - Registry outage warnings remain non-blocking; see `feeDiscountRegistryWarning.ts`.
 */

export const FEE_DISCOUNT_UNREGISTERED_CTA_TEXT = 'Hold CL8Y & register →'

export const FEE_DISCOUNT_ELIGIBILITY_NOTE = 'Hold CL8Y and register on Tiers for a discount.'
