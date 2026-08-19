import { queryContract } from './queries'
import type { ObserveResponse, OracleInfoResponse } from '@/types'

export type { OracleInfoResponse }

const DECIMAL_SCALE = BigInt('1000000000000000000') // 1e18

export async function observe(pairAddress: string, secondsAgo: number[]): Promise<ObserveResponse> {
  return queryContract<ObserveResponse>(pairAddress, {
    observe: { seconds_ago: secondsAgo },
  })
}

export async function getOracleInfo(pairAddress: string): Promise<OracleInfoResponse> {
  return queryContract<OracleInfoResponse>(pairAddress, {
    oracle_info: {},
  })
}

/**
 * Arithmetic-mean TWAP as a **raw** Decimal string (token1 base units per token0 base unit).
 * Same units as on-chain `compute_twap_price` / limit `price` — Charts must human-scale with
 * `raw × 10^(d0 − d1)` (GitLab #564). Zero / inverted cum / bad window → `null`.
 */
export function computeTwapPriceDecimalString(cumStart: bigint, cumEnd: bigint, timeElapsed: number): string | null {
  if (!Number.isInteger(timeElapsed) || timeElapsed <= 0) return null
  if (cumEnd < cumStart) return null
  const diff = cumEnd - cumStart
  const avgScaled = diff / BigInt(timeElapsed)
  if (avgScaled <= 0n) return null
  const intPart = avgScaled / DECIMAL_SCALE
  const fracPart = avgScaled % DECIMAL_SCALE
  const frac = fracPart.toString().padStart(18, '0').replace(/0+$/, '')
  return frac ? `${intPart.toString()}.${frac}` : intPart.toString()
}

export function computeTwapPrice(cumStart: bigint, cumEnd: bigint, timeElapsed: number): number {
  const decimal = computeTwapPriceDecimalString(cumStart, cumEnd, timeElapsed)
  if (decimal == null) return 0
  const n = Number(decimal)
  return Number.isFinite(n) ? n : 0
}

export type TwapWindowPrice = { label: string; seconds: number; price: string | null }

export async function getTwapPrices(
  pairAddress: string,
  windows: { label: string; seconds: number }[]
): Promise<TwapWindowPrice[]> {
  const secondsAgo = [0, ...windows.map((w) => w.seconds)]

  try {
    const resp = await observe(pairAddress, secondsAgo)
    const cumsA = resp.price_a_cumulatives.map((c) => BigInt(c))
    const nowCumA = cumsA[0]

    return windows.map((w, i) => {
      try {
        const pastCumA = cumsA[i + 1]
        const price = computeTwapPriceDecimalString(pastCumA, nowCumA, w.seconds)
        return { ...w, price }
      } catch {
        return { ...w, price: null }
      }
    })
  } catch {
    return windows.map((w) => ({ ...w, price: null }))
  }
}
