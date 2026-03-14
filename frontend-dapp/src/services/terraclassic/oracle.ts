import { queryContract } from './queries'
import type { ObserveResponse, OracleInfoResponse } from '@/types'

export type { OracleInfoResponse }

const DECIMAL_SCALE = BigInt('1000000000000000000') // 1e18

export async function observe(
  pairAddress: string,
  secondsAgo: number[]
): Promise<ObserveResponse> {
  return queryContract<ObserveResponse>(pairAddress, {
    observe: { seconds_ago: secondsAgo },
  })
}

export async function getOracleInfo(pairAddress: string): Promise<OracleInfoResponse> {
  return queryContract<OracleInfoResponse>(pairAddress, {
    oracle_info: {},
  })
}

export function computeTwapPrice(
  cumStart: bigint,
  cumEnd: bigint,
  timeElapsed: number
): number {
  if (timeElapsed === 0) return 0
  if (cumEnd < cumStart) return 0
  const diff = cumEnd - cumStart
  const avgScaled = diff / BigInt(timeElapsed)
  // Split into integer and fractional parts to avoid precision loss for large values
  const intPart = avgScaled / DECIMAL_SCALE
  const fracPart = avgScaled % DECIMAL_SCALE
  return Number(intPart) + Number(fracPart) / Number(DECIMAL_SCALE)
}

export async function getTwapPrices(
  pairAddress: string,
  windows: { label: string; seconds: number }[]
): Promise<{ label: string; seconds: number; price: number | null }[]> {
  const secondsAgo = [0, ...windows.map((w) => w.seconds)]

  try {
    const resp = await observe(pairAddress, secondsAgo)
    const cumsA = resp.price_a_cumulatives.map((c) => BigInt(c))
    const nowCumA = cumsA[0]

    return windows.map((w, i) => {
      try {
        const pastCumA = cumsA[i + 1]
        const price = computeTwapPrice(pastCumA, nowCumA, w.seconds)
        return { ...w, price }
      } catch {
        return { ...w, price: null }
      }
    })
  } catch {
    return windows.map((w) => ({ ...w, price: null }))
  }
}
