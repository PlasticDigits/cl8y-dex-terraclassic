import { formatTokenAmount } from '@/utils/formatAmount'
import { estimateFeeUlunaAmountForGasLimit, gasLimitForLimitOrderBatch } from '@/services/terraclassic/terraGas'
import type { LimitLadderPlacementPlan } from '@/utils/limitLadderPlacementPlan'

/** Human-readable batch vs N×single place gas comparison (GitLab #206). */
export function formatLimitBatchGasSavingsLine(rungCount: number, batchMinUluna: bigint, savingsUluna: bigint): string {
  const batchApprox = formatTokenAmount(batchMinUluna.toString(), 6, 4)
  if (rungCount < 2 || savingsUluna <= 0n) {
    return `One transaction after allowance · est. min gas ~${batchApprox} LUNC`
  }
  const savedApprox = formatTokenAmount(savingsUluna.toString(), 6, 4)
  return `One tx after allowance · est. ~${batchApprox} LUNC gas (saves ~${savedApprox} LUNC vs ${rungCount} separate placements)`
}

/** Pre-submit ladder summary: path, expected rungs, gas model (GitLab #268). */
export function formatLimitLadderPlacementSummary(
  rungCount: number,
  maxAdjustSteps: number,
  plan: LimitLadderPlacementPlan | null | undefined
): string {
  const gasLimit = gasLimitForLimitOrderBatch(rungCount)
  const gasUluna = estimateFeeUlunaAmountForGasLimit(gasLimit)
  const gasApprox = formatTokenAmount(gasUluna.toString(), 6, 4)
  const pathLabel =
    plan?.path === 'deep_batch'
      ? 'hinted batch'
      : plan?.path === 'single_anchor_ladder'
        ? 'ladder (single anchor)'
        : 'ladder (thin book)'
  const placed = plan != null ? `${plan.skipRisk.predictedPlaced}/${rungCount} rungs expected` : `${rungCount} rungs`
  const stepsNote = plan ? ` · max steps ${maxAdjustSteps} (recommended ${plan.recommendedMaxSteps})` : ''
  return `${pathLabel} · ${placed} · est. gas ~${gasApprox} LUNC${stepsNote}`
}
