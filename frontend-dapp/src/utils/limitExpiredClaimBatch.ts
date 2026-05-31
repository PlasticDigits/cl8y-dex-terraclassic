import {
  CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT,
  estimateFeeUlunaAmountForGasLimit,
  gasLimitForLimitOrderCancelBatch,
} from '@/services/terraclassic/terraGas'
import { MAX_LIMIT_BATCH_RUNGS_HARD_CAP } from '@/utils/constants'
import { formatTokenAmount } from '@/utils/formatAmount'

export type LimitExpiredClaimInput = number | number[]

/** Dedupe and validate order ids before single or batch claim (GitLab #253). */
export function normalizeExpiredClaimOrderIds(input: LimitExpiredClaimInput): number[] {
  const raw = Array.isArray(input) ? input : [input]
  const ids = [...new Set(raw.filter((id) => Number.isFinite(id) && id >= 1))].sort((a, b) => a - b)
  if (ids.length === 0) {
    throw new Error('Invalid order id')
  }
  return ids
}

/** Split ids into on-chain batch chunks (≤ {@link MAX_LIMIT_BATCH_RUNGS_HARD_CAP} per tx). */
export function chunkExpiredClaimOrderIds(orderIds: number[], maxChunk = MAX_LIMIT_BATCH_RUNGS_HARD_CAP): number[][] {
  if (maxChunk < 1) throw new Error('Invalid batch chunk size')
  const chunks: number[][] = []
  for (let i = 0; i < orderIds.length; i += maxChunk) {
    chunks.push(orderIds.slice(i, i + maxChunk))
  }
  return chunks
}

/** Batch vs N×single claim fee line for confirm copy (GitLab #259 — same math as broadcast). */
export function formatExpiredClaimBatchGasLine(chunkSize: number): string {
  const n = Math.max(1, chunkSize)
  const batchGas = gasLimitForLimitOrderCancelBatch(n)
  const batchUluna = estimateFeeUlunaAmountForGasLimit(batchGas)
  const batchApprox = formatTokenAmount(batchUluna.toString(), 6, 4)

  if (n < 2) {
    return `Est. ~${batchApprox} LUNC gas.`
  }

  const singlesUluna = estimateFeeUlunaAmountForGasLimit(CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT) * BigInt(n)
  const savingsUluna = singlesUluna - batchUluna
  if (savingsUluna <= 0n) {
    return `Est. ~${batchApprox} LUNC gas.`
  }

  const savedApprox = formatTokenAmount(savingsUluna.toString(), 6, 4)
  return `Est. ~${batchApprox} LUNC gas (saves ~${savedApprox} LUNC vs ${n} separate claims).`
}

export function confirmExpiredClaimBatchMessage(
  chunk: number[],
  chunkIndex: number,
  totalChunks: number,
  totalOrderCount: number
): string {
  const gasLine = formatExpiredClaimBatchGasLine(chunk.length)
  if (totalChunks === 1) {
    return `Claim all ${chunk.length} expired refund(s) in one transaction? ${gasLine}`
  }
  return (
    `Claim batch ${chunkIndex + 1} of ${totalChunks}: ${chunk.length} expired refund(s) in one transaction? ` +
    `(${totalOrderCount} total) ${gasLine}`
  )
}

export function isOrderIdInExpiredClaimVariables(
  orderId: number,
  variables: LimitExpiredClaimInput | undefined
): boolean {
  if (variables === undefined) return false
  if (Array.isArray(variables)) return variables.includes(orderId)
  return variables === orderId
}
