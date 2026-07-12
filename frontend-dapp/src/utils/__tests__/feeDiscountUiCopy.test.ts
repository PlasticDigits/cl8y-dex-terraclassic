import { describe, it, expect } from 'vitest'
import { FEE_DISCOUNT_ELIGIBILITY_NOTE, FEE_DISCOUNT_UNREGISTERED_CTA_TEXT } from '../feeDiscountUiCopy'

describe('feeDiscountUiCopy (GitLab #476)', () => {
  it('CTA keeps Hold CL8Y wording and points users to register', () => {
    expect(FEE_DISCOUNT_UNREGISTERED_CTA_TEXT).toMatch(/Hold CL8Y/i)
    expect(FEE_DISCOUNT_UNREGISTERED_CTA_TEXT).toMatch(/register/i)
  })

  it('eligibility note requires hold + register and rejects lookalike tokens', () => {
    expect(FEE_DISCOUNT_ELIGIBILITY_NOTE).toMatch(/configured CL8Y CW20/i)
    expect(FEE_DISCOUNT_ELIGIBILITY_NOTE).toMatch(/register/i)
    expect(FEE_DISCOUNT_ELIGIBILITY_NOTE).toMatch(/do not count/i)
  })
})
