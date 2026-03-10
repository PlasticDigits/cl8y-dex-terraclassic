import { queryContract } from './queries'
import type { ObserveResponse, OracleInfoResponse } from '@/types'

const Q64 = BigInt(1) << BigInt(64)

/**
 * Query TWAP tick cumulatives at specified time offsets.
 * seconds_ago[0] = most recent, seconds_ago[n] = furthest back.
 */
export async function observe(
  pairAddress: string,
  secondsAgo: number[]
): Promise<ObserveResponse> {
  return queryContract<ObserveResponse>(pairAddress, {
    observe: { seconds_ago: secondsAgo },
  })
}

/** Query oracle ring buffer metadata. */
export async function getOracleInfo(pairAddress: string): Promise<OracleInfoResponse> {
  return queryContract<OracleInfoResponse>(pairAddress, {
    oracle_info: {},
  })
}

/**
 * Compute geometric-mean TWAP price from two tick cumulatives.
 * Formula: price = 2^((tick_end - tick_start) / time_elapsed)
 * where ticks are in Q64.64 fixed-point.
 */
export function computeTwapPrice(
  tickCumulativeStart: bigint,
  tickCumulativeEnd: bigint,
  timeElapsed: number
): number {
  if (timeElapsed === 0) return 0
  const avgTickQ64 = (tickCumulativeEnd - tickCumulativeStart) / BigInt(timeElapsed)
  return exp2TickToNumber(avgTickQ64)
}

/**
 * Compute 2^(tick/2^64) as a JS number.
 * Approximation using: 2^x = e^(x * ln2), Taylor series for e^x.
 */
function exp2TickToNumber(tickQ64: bigint): number {
  const negative = tickQ64 < 0n
  const absVal = negative ? -tickQ64 : tickQ64

  const intPart = Number(absVal >> BigInt(64))
  const fracQ64 = absVal & (Q64 - 1n)

  const frac = Number(fracQ64) / Number(Q64)
  const x = frac * 0.6931471805599453 // ln(2)

  let expX = 1.0
  let term = 1.0
  for (let i = 1; i <= 12; i++) {
    term *= x / i
    expX += term
  }

  let price = expX * Math.pow(2, intPart)
  if (negative) price = 1 / price
  return price
}

/**
 * Fetch TWAP prices at multiple time windows for a pair.
 * Returns an object with prices at each window (e.g., 5m, 1h, 24h).
 */
export async function getTwapPrices(
  pairAddress: string,
  windows: { label: string; seconds: number }[]
): Promise<{ label: string; seconds: number; price: number | null }[]> {
  const secondsAgo = [0, ...windows.map((w) => w.seconds)]

  try {
    const resp = await observe(pairAddress, secondsAgo)
    const ticks = resp.tick_cumulatives.map((t) => BigInt(t))
    const nowTick = ticks[0]

    return windows.map((w, i) => {
      try {
        const pastTick = ticks[i + 1]
        const price = computeTwapPrice(pastTick, nowTick, w.seconds)
        return { ...w, price }
      } catch {
        return { ...w, price: null }
      }
    })
  } catch {
    return windows.map((w) => ({ ...w, price: null }))
  }
}
