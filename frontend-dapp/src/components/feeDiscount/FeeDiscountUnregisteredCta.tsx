import { FEE_DISCOUNT_ELIGIBILITY_NOTE, FEE_DISCOUNT_UNREGISTERED_CTA_TEXT } from '@/utils/feeDiscountUiCopy'

/**
 * Relative `/tiers` CTA for connected unregistered wallets (GitLab #476).
 * Holding alone does not apply a discount — users must register the eligible CL8Y CW20.
 */
export function FeeDiscountUnregisteredCta({
  testId = 'fee-discount-unregistered-cta',
  className = '',
}: {
  testId?: string
  className?: string
}) {
  return (
    <div
      data-testid={testId}
      className={`p-2 border-2 rounded-none text-xs shadow-[1px_1px_0_#000] ${className}`.trim()}
      style={{
        borderColor: 'color-mix(in srgb, var(--cyan) 30%, transparent)',
        background: 'color-mix(in srgb, var(--cyan) 5%, transparent)',
        color: 'var(--cyan)',
      }}
      title={FEE_DISCOUNT_ELIGIBILITY_NOTE}
    >
      <a href="/tiers" className="hover:underline uppercase tracking-wide font-semibold">
        {FEE_DISCOUNT_UNREGISTERED_CTA_TEXT}
      </a>
    </div>
  )
}
