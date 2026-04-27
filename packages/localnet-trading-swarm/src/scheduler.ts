/** Exponential inter-arrival with mean `meanSeconds` (Poisson rate λ = 1/mean). */
export function sampleInterTxDelaySeconds(meanSeconds: number, jitterMaxSeconds = 2): number {
  const u = Math.max(Number.MIN_VALUE, Math.random())
  const exp = -Math.log(u) * meanSeconds
  const jitter = jitterMaxSeconds > 0 ? Math.random() * jitterMaxSeconds : 0
  return exp + jitter
}

export function createTxQueue(): <T>(fn: () => Promise<T>) => Promise<T> {
  let tail: Promise<unknown> = Promise.resolve()
  return function enqueue<T>(fn: () => Promise<T>): Promise<T> {
    const next = tail.then(() => fn())
    tail = next.then(() => {}).catch(() => {})
    return next as Promise<T>
  }
}

export class GapAccumulator {
  private readonly gaps: number[][]

  constructor(botCount: number) {
    this.gaps = Array.from({ length: botCount }, () => [])
  }

  push(botIndex: number, gapSec: number): void {
    this.gaps[botIndex]!.push(gapSec)
  }

  summary(meanTarget: number): {
    bot: number
    samples: number
    meanGapSec: number | null
    deltaFromTarget: number | null
  }[] {
    return this.gaps.map((arr, bot) => {
      if (arr.length === 0) {
        return { bot, samples: 0, meanGapSec: null, deltaFromTarget: null }
      }
      const meanGapSec = arr.reduce((a, b) => a + b, 0) / arr.length
      return {
        bot,
        samples: arr.length,
        meanGapSec,
        deltaFromTarget: meanGapSec - meanTarget,
      }
    })
  }
}
