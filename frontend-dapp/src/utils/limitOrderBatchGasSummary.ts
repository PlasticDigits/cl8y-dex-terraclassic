import { formatTokenAmount } from '@/utils/formatAmount'

/** Human-readable batch vs N×single place gas comparison (GitLab #206). */
export function formatLimitBatchGasSavingsLine(rungCount: number, batchMinUluna: bigint, savingsUluna: bigint): string {
  const batchApprox = formatTokenAmount(batchMinUluna.toString(), 6, 4)
  if (rungCount < 2 || savingsUluna <= 0n) {
    return `One transaction after allowance · est. min gas ~${batchApprox} LUNC`
  }
  const savedApprox = formatTokenAmount(savingsUluna.toString(), 6, 4)
  return `One tx after allowance · est. ~${batchApprox} LUNC gas (saves ~${savedApprox} LUNC vs ${rungCount} separate placements)`
}
