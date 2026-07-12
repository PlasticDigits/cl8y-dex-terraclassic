import { FEE_DISCOUNT_REGISTRY_WARNING_TEXT } from '@/utils/feeDiscountRegistryWarning'

/** Non-blocking amber banner when the fee-discount registry may be unreachable (GitLab #374 / #476). */
export function FeeDiscountRegistryWarning({ testId }: { testId: string }) {
  return (
    <div className="alert-warning text-sm mb-4" role="status" data-testid={testId}>
      {FEE_DISCOUNT_REGISTRY_WARNING_TEXT}
    </div>
  )
}
