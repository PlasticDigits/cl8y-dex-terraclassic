import { MAX_LIMIT_BATCH_RUNGS_HARD_CAP } from '@/utils/constants'

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

export function confirmExpiredClaimBatchMessage(
  chunk: number[],
  chunkIndex: number,
  totalChunks: number,
  totalOrderCount: number
): string {
  if (totalChunks === 1) {
    return `Claim all ${chunk.length} expired refund(s) in one transaction?`
  }
  return (
    `Claim batch ${chunkIndex + 1} of ${totalChunks}: ${chunk.length} expired refund(s) in one transaction? ` +
    `(${totalOrderCount} total)`
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
