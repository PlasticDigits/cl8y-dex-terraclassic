import { parseDecimalStringToScaled } from '@/utils/swapMaxSpread'

function decimal(numerator: bigint, digits: number): string {
  const raw = numerator.toString().padStart(digits + 1, '0')
  const whole = digits ? raw.slice(0, -digits) : raw
  const fraction = digits ? raw.slice(-digits).replace(/0+$/, '') : ''
  return fraction ? `${whole}.${fraction}` : whole
}

/** Exact decimal percent→ratio, avoiding binary division artifacts in signed/request constraints. */
export function slippagePercentToMaxSpread(percent: number): string {
  const text = String(percent)
  const match = /^(\d+)(?:\.(\d{1,16}))?$/.exec(text)
  if (!match) throw new Error('Invalid slippage protection.')
  const { mantissa, scalePow } = parseDecimalStringToScaled(text)
  if (mantissa <= 0n || mantissa > 50n * scalePow) throw new Error('Invalid slippage protection.')
  return decimal(BigInt(match[1] + (match[2] ?? '')), (match[2]?.length ?? 0) + 2)
}

/** Rational equivalence of the configured percent and execution ratio; both remain bounded. */
export function spreadMatchesSlippagePercent(spread: string, percent: number): boolean {
  try {
    const expected = slippagePercentToMaxSpread(percent)
    if (!/^0(?:\.\d{1,18})?$/.test(spread)) return false
    const actual = parseDecimalStringToScaled(spread)
    const wanted = parseDecimalStringToScaled(expected)
    return actual.mantissa > 0n && actual.mantissa * wanted.scalePow === wanted.mantissa * actual.scalePow
  } catch {
    return false
  }
}

export function maxSpreadToSlippagePercent(spread: string): number {
  const match = /^(\d+)(?:\.(\d{1,18}))?$/.exec(spread)
  if (!match) throw new Error('Invalid slippage protection.')
  const fraction = match[2] ?? ''
  const percent = Number(decimal(BigInt(match[1] + fraction) * 100n, fraction.length))
  slippagePercentToMaxSpread(percent)
  return percent
}
