import { describe, expect, it } from 'vitest'
import {
  FEE_DISCOUNT_REGISTRY_WARNING_TEXT,
  resolveFeeDiscountRegistryStatus,
  shouldShowFeeDiscountRegistryWarning,
} from '@/utils/feeDiscountRegistryWarning'

describe('feeDiscountRegistryWarning', () => {
  it('exposes stable warning copy without LCD or address details', () => {
    expect(FEE_DISCOUNT_REGISTRY_WARNING_TEXT).toMatch(/full pair fee/i)
    expect(FEE_DISCOUNT_REGISTRY_WARNING_TEXT).not.toMatch(/terra1|@|error/i)
  })

  describe('resolveFeeDiscountRegistryStatus', () => {
    it('returns unregistered when registration succeeded and wallet is not registered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          registrationSucceeded: true,
          registered: false,
          registrationQueryError: false,
          discountQueryError: false,
        })
      ).toBe('unregistered')
    })

    it('returns registered when registration succeeded and wallet is registered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          registrationSucceeded: true,
          registered: true,
          registrationQueryError: false,
          discountQueryError: false,
        })
      ).toBe('registered')
    })

    it('returns registry_unreachable when registration LCD query fails', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          registrationSucceeded: false,
          registered: false,
          registrationQueryError: true,
          discountQueryError: false,
        })
      ).toBe('registry_unreachable')
    })

    it('returns registry_unreachable when discount LCD query fails for a registered wallet', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          registrationSucceeded: true,
          registered: true,
          registrationQueryError: false,
          discountQueryError: true,
        })
      ).toBe('registry_unreachable')
    })

    it('returns unregistered when only discount fails but registration succeeded as unregistered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          registrationSucceeded: true,
          registered: false,
          registrationQueryError: false,
          discountQueryError: true,
        })
      ).toBe('unregistered')
    })

    it('returns registry_unreachable when indexer reports registry down', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          registrationSucceeded: true,
          registered: true,
          registrationQueryError: false,
          discountQueryError: false,
          indexerHealth: { configured: true, fee_discount_registry_ok: false },
        })
      ).toBe('registry_unreachable')
    })

    it('keeps unregistered when indexer reports registry down but LCD registration read succeeded as unregistered', () => {
      expect(
        resolveFeeDiscountRegistryStatus({
          registrationSucceeded: true,
          registered: false,
          registrationQueryError: false,
          discountQueryError: false,
          indexerHealth: { configured: true, fee_discount_registry_ok: false },
        })
      ).toBe('unregistered')
    })
  })

  describe('shouldShowFeeDiscountRegistryWarning', () => {
    it('shows only for registry_unreachable', () => {
      expect(shouldShowFeeDiscountRegistryWarning('registry_unreachable')).toBe(true)
      expect(shouldShowFeeDiscountRegistryWarning('unregistered')).toBe(false)
      expect(shouldShowFeeDiscountRegistryWarning('registered')).toBe(false)
    })
  })
})
