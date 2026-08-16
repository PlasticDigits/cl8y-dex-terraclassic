/**
 * Human ↔ raw limit-price scale (GitLab #529).
 *
 * On-chain `price` is raw token1 base units per token0 base unit (used in `fill × price`).
 * Human token1-per-token0 is `raw × 10^(decimals0 − decimals1)`.
 * UI, gates, and refs stay human; convert at the chain / indexer edge.
 */

export type LimitPriceDecimals = {
  decimals0: number
  decimals1: number
}

function assertFiniteDecimals(d0: number, d1: number): void {
  if (!Number.isInteger(d0) || !Number.isInteger(d1) || d0 < 0 || d1 < 0 || d0 > 18 || d1 > 18) {
    throw new Error('limit price decimals must be integers in 0…18')
  }
}

/**
 * Multiply a positive decimal string by `10^exp` (exp may be negative) without floats.
 */
export function scaleDecimalStringByPow10(value: string, exp: number): string {
  const trimmed = value.trim()
  if (!trimmed || trimmed.startsWith('-') || trimmed.startsWith('+')) {
    throw new Error('limit price must be a positive decimal string')
  }
  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    throw new Error('limit price must be a positive decimal string')
  }
  if (!Number.isInteger(exp)) {
    throw new Error('limit price scale exponent must be an integer')
  }

  const [intRaw, fracRaw = ''] = trimmed.split('.')
  const intPart = (intRaw || '0').replace(/^0+/, '') || '0'
  const fracPart = fracRaw.replace(/0+$/, '')

  if (exp === 0) {
    return fracPart ? `${intPart}.${fracPart}` : intPart
  }

  if (exp > 0) {
    const frac = fracPart.padEnd(exp, '0')
    const take = frac.slice(0, exp)
    const rest = frac.slice(exp).replace(/0+$/, '')
    const combined = `${intPart}${take}`.replace(/^0+/, '') || '0'
    return rest ? `${combined}.${rest}` : combined
  }

  const shift = -exp
  const digits = `${intPart}${fracPart}`.replace(/^0+/, '') || '0'
  if (digits === '0') return '0'
  if (intPart.length > shift) {
    const split = intPart.length - shift
    const nextInt = intPart.slice(0, split).replace(/^0+/, '') || '0'
    const nextFrac = `${intPart.slice(split)}${fracPart}`.replace(/0+$/, '')
    return nextFrac ? `${nextInt}.${nextFrac}` : nextInt
  }
  const pad = shift - intPart.length
  const nextFrac = `${'0'.repeat(pad)}${intPart}${fracPart}`.replace(/0+$/, '')
  return nextFrac ? `0.${nextFrac}` : '0'
}

/** raw = human × 10^(decimals1 − decimals0) */
export function humanLimitPriceToRaw(human: string, decimals0: number, decimals1: number): string {
  assertFiniteDecimals(decimals0, decimals1)
  return scaleDecimalStringByPow10(human, decimals1 - decimals0)
}

/** human = raw × 10^(decimals0 − decimals1) */
export function rawLimitPriceToHuman(raw: string, decimals0: number, decimals1: number): string {
  assertFiniteDecimals(decimals0, decimals1)
  return scaleDecimalStringByPow10(raw, decimals0 - decimals1)
}

export function scaleHumanLimitPriceForChain(human: string, scale?: LimitPriceDecimals | null): string {
  if (!scale || scale.decimals0 === scale.decimals1) return human.trim()
  return humanLimitPriceToRaw(human, scale.decimals0, scale.decimals1)
}

export function scaleRawLimitPriceForDisplay(raw: string, scale?: LimitPriceDecimals | null): string {
  if (!raw?.trim()) return raw
  if (!scale || scale.decimals0 === scale.decimals1) return raw.trim()
  try {
    return rawLimitPriceToHuman(raw, scale.decimals0, scale.decimals1)
  } catch {
    return raw.trim()
  }
}

export function limitPriceDecimalsFromPair(
  pair:
    | {
        asset_0?: { decimals?: number }
        asset_1?: { decimals?: number }
      }
    | null
    | undefined
): LimitPriceDecimals | null {
  const d0 = pair?.asset_0?.decimals
  const d1 = pair?.asset_1?.decimals
  if (d0 == null || d1 == null || !Number.isFinite(d0) || !Number.isFinite(d1)) return null
  return { decimals0: d0, decimals1: d1 }
}
