/**
 * Shared fee-discount UX copy for Swap and Pool (GitLab #476).
 *
 * Invariants:
 * - Discount requires **hold** of the configured `cl8y_token` CW20 **and** `Register` on `/tiers`.
 * - Holding alone (or a differently named/bridged asset) does not apply a discount.
 * - CTA must stay a same-origin relative `/tiers` link (no external phishing targets).
 * - Registry outage warnings remain non-blocking; see `feeDiscountRegistryWarning.ts`.
 */

export const FEE_DISCOUNT_UNREGISTERED_CTA_TEXT = 'Hold CL8Y & register to reduce fees →'

export const FEE_DISCOUNT_ELIGIBILITY_NOTE =
  'Discount applies only after you hold the configured CL8Y CW20 and register a tier on /tiers. Other tokens with similar names do not count.'
