import type { DiscountResponse, RegistrationResponse } from '@/types'

export type FeeDiscountRegistryStatus = 'unconfigured' | 'unregistered' | 'registered' | 'registry_unreachable'

export interface FeeDiscountHealthSnapshot {
  configured: boolean
  fee_discount_registry_ok: boolean | null
  consecutive_lcd_failures: number
}

export interface FeeDiscountRegistryWarningInput {
  feeDiscountContractConfigured: boolean
  registration?: RegistrationResponse | null
  discount?: DiscountResponse | null
  registrationQueryError: boolean
  discountQueryError: boolean
  indexerHealth?: FeeDiscountHealthSnapshot | null
}

/**
 * Distinguish "not registered" from "registry LCD unreachable" where data allows (GitLab #365).
 * Returns `registry_unreachable` when a registered trader cannot read discount state or ops health is down.
 */
export function resolveFeeDiscountRegistryStatus(input: FeeDiscountRegistryWarningInput): FeeDiscountRegistryStatus {
  if (!input.feeDiscountContractConfigured) {
    return 'unconfigured'
  }

  const indexerDown = input.indexerHealth?.configured === true && input.indexerHealth.fee_discount_registry_ok === false
  const lcdUnreachable = input.registrationQueryError || input.discountQueryError

  if (indexerDown || lcdUnreachable) {
    return 'registry_unreachable'
  }

  if (!input.registration?.registered) {
    return 'unregistered'
  }

  return 'registered'
}

/** Non-blocking trader warning when registry outage may charge full fee despite registration. */
export function shouldShowFeeDiscountRegistryWarning(input: FeeDiscountRegistryWarningInput): boolean {
  return resolveFeeDiscountRegistryStatus(input) === 'registry_unreachable'
}

export const FEE_DISCOUNT_REGISTRY_WARNING_TEXT =
  'Fee discount unavailable; full pair fee may apply until the registry is reachable again.'
