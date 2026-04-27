import { describe, it, expect } from 'vitest'
import { sampleInterTxDelaySeconds, GapAccumulator, createTxQueue } from './scheduler.js'

describe('sampleInterTxDelaySeconds', () => {
  it('has sample mean near target over many draws', () => {
    const mean = 20
    const n = 4000
    let sum = 0
    for (let i = 0; i < n; i++) {
      sum += sampleInterTxDelaySeconds(mean, 0)
    }
    const sampleMean = sum / n
    expect(sampleMean).toBeGreaterThan(mean * 0.85)
    expect(sampleMean).toBeLessThan(mean * 1.25)
  })
})

describe('GapAccumulator', () => {
  it('computes per-bot means', () => {
    const g = new GapAccumulator(2)
    g.push(0, 18)
    g.push(0, 22)
    g.push(1, 20)
    const s = g.summary(20)
    expect(s[0]!.samples).toBe(2)
    expect(s[0]!.meanGapSec).toBeCloseTo(20, 5)
    expect(s[1]!.meanGapSec).toBe(20)
  })
})

describe('createTxQueue', () => {
  it('serializes async work per queue', async () => {
    const q = createTxQueue()
    const order: number[] = []
    const p1 = q(async () => {
      order.push(1)
      await new Promise((r) => setTimeout(r, 20))
    })
    const p2 = q(async () => {
      order.push(2)
    })
    await Promise.all([p1, p2])
    expect(order).toEqual([1, 2])
  })
})
