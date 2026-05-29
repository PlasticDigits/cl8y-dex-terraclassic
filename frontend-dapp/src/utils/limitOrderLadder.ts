/**
 * Client-side ladder expansion aligned with `dex_common::limit_placement::expand_limit_ladder`.
 */

export type LimitLadderDistribution = 'equal'

export interface LimitLadderSpec {
  side: 'bid' | 'ask'
  startPrice: string
  endPrice: string
  count: number
  totalAmountRaw: string
  distribution: LimitLadderDistribution
  maxAdjustSteps: number
  expiresAt?: number | null
}

export interface LadderRungPreview {
  price: string
  amountRaw: string
}

export class LimitLadderError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'LimitLadderError'
  }
}

function parsePositiveDecimal(s: string): number {
  const n = Number.parseFloat(s)
  if (!(n > 0) || !Number.isFinite(n)) {
    throw new LimitLadderError('prices must be positive numbers')
  }
  return n
}

function parsePositiveInt(s: string): bigint {
  try {
    const v = BigInt(s)
    if (v <= 0n) throw new Error()
    return v
  } catch {
    throw new LimitLadderError('total amount must be a positive integer string')
  }
}

/** Expand ladder spec into per-rung raw amounts and human prices (for preview / submit). */
export function expandLimitLadder(spec: LimitLadderSpec, maxRungs: number): LadderRungPreview[] {
  if (spec.count < 2) {
    throw new LimitLadderError('ladder count must be at least 2')
  }
  if (spec.count > maxRungs) {
    throw new LimitLadderError(`ladder count exceeds pair max_batch_rungs (${maxRungs})`)
  }

  const start = parsePositiveDecimal(spec.startPrice)
  const end = parsePositiveDecimal(spec.endPrice)
  const total = parsePositiveInt(spec.totalAmountRaw)

  const prices: number[] = []
  if (spec.count === 1) {
    prices.push(start)
  } else {
    for (let i = 0; i < spec.count; i++) {
      if (i === 0) prices.push(start)
      else if (i === spec.count - 1) prices.push(end)
      else prices.push(start + ((end - start) * i) / (spec.count - 1))
    }
  }

  const base = total / BigInt(spec.count)
  const amounts: bigint[] = Array.from({ length: spec.count }, () => base)
  const assigned = base * BigInt(spec.count)
  const remainder = total - assigned
  if (remainder > 0n) {
    amounts[amounts.length - 1] = amounts[amounts.length - 1]! + remainder
  }

  return prices.map((price, i) => ({
    price: String(price),
    amountRaw: amounts[i]!.toString(),
  }))
}

export function sumLadderAmountsRaw(rungs: LadderRungPreview[]): string {
  return rungs.reduce((acc, r) => acc + BigInt(r.amountRaw), 0n).toString()
}
