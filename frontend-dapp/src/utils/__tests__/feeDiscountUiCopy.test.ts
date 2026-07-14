import { describe, it, expect } from 'vitest'
import { FEE_DISCOUNT_ELIGIBILITY_NOTE, FEE_DISCOUNT_UNREGISTERED_CTA_TEXT } from '../feeDiscountUiCopy'

describe('feeDiscountUiCopy (GitLab #476)', () => {
  it('CTA keeps Hold CL8Y wording and points users to register', () => {
    expect(FEE_DISCOUNT_UNREGISTERED_CTA_TEXT).toMatch(/Hold CL8Y/i)
    expect(FEE_DISCOUNT_UNREGISTERED_CTA_TEXT).toMatch(/register/i)
  })

  it('eligibility note requires hold + register', () => {
    expect(FEE_DISCOUNT_ELIGIBILITY_NOTE).toMatch(/CL8Y/i)
    expect(FEE_DISCOUNT_ELIGIBILITY_NOTE).toMatch(/register/i)
    expect(FEE_DISCOUNT_ELIGIBILITY_NOTE.split(/\s+/).length).toBeLessThanOrEqual(10)
  })
})
