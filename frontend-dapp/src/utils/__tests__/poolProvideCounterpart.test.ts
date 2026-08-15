import { describe, it, expect } from 'vitest'
import { computeProvideCounterpartHuman } from '../poolProvideCounterpart'
import type { PoolResponse } from '@/types'

const pool1m2m = (): PoolResponse => ({
  assets: [
    { info: { token: { contract_addr: 'a' } }, amount: '1000000' },
    { info: { token: { contract_addr: 'b' } }, amount: '2000000' },
  ],
  total_share: '2000000',
})

describe('computeProvideCounterpartHuman', () => {
  it('auto-fills B from A on 1:2 pool (6 decimals)', () => {
    expect(
      computeProvideCounterpartHuman({
        editedSide: 'a',
        editedHuman: '1',
        pool: pool1m2m(),
        decimalsA: 6,
        decimalsB: 6,
        needsWrapA: false,
        needsWrapB: false,
      })
    ).toBe('2')
  })

  it('auto-fills A from B when editing B', () => {
    expect(
      computeProvideCounterpartHuman({
        editedSide: 'b',
        editedHuman: '2',
        pool: pool1m2m(),
        decimalsA: 6,
        decimalsB: 6,
        needsWrapA: false,
        needsWrapB: false,
      })
    ).toBe('1')
  })

  it('returns null for empty pool', () => {
    const empty: PoolResponse = {
      assets: [
        { info: { token: { contract_addr: 'a' } }, amount: '0' },
        { info: { token: { contract_addr: 'b' } }, amount: '0' },
      ],
      total_share: '0',
    }
    expect(
      computeProvideCounterpartHuman({
        editedSide: 'a',
        editedHuman: '1',
        pool: empty,
        decimalsA: 6,
        decimalsB: 6,
        needsWrapA: false,
        needsWrapB: false,
      })
    ).toBeNull()
  })

  it('returns null for draft amounts', () => {
    expect(
      computeProvideCounterpartHuman({
        editedSide: 'a',
        editedHuman: '.',
        pool: pool1m2m(),
        decimalsA: 6,
        decimalsB: 6,
        needsWrapA: false,
        needsWrapB: false,
      })
    ).toBeNull()
  })

  it('uses post–mapper-fee net when native wrap is enabled (#512 fee-only mint)', () => {
    const withoutWrap = computeProvideCounterpartHuman({
      editedSide: 'a',
      editedHuman: '1',
      pool: pool1m2m(),
      decimalsA: 6,
      decimalsB: 6,
      needsWrapA: false,
      needsWrapB: false,
    })
    const withWrap = computeProvideCounterpartHuman({
      editedSide: 'a',
      editedHuman: '1',
      pool: pool1m2m(),
      decimalsA: 6,
      decimalsB: 6,
      needsWrapA: true,
      needsWrapB: false,
      wrapMapperFeeBps: 200,
    })
    expect(withoutWrap).toBe('2')
    expect(withWrap).not.toBe(withoutWrap)
    // 1 * 0.98 fee net → counterpart ~1.96 on 1:2 pool
    expect(Number(withWrap)).toBeGreaterThan(1.9)
    expect(Number(withWrap)).toBeLessThan(2)
  })

  it('does not require tax params for wrap-side auto-fill (#512)', () => {
    expect(
      computeProvideCounterpartHuman({
        editedSide: 'a',
        editedHuman: '1',
        pool: pool1m2m(),
        decimalsA: 6,
        decimalsB: 6,
        needsWrapA: true,
        needsWrapB: false,
        wrapMapperFeeBps: 0,
      })
    ).toBe('2')
  })

  it('applies wrap-mapper wrap fee when auto-filling (#507 / #512 / #516)', () => {
    const feeFree = computeProvideCounterpartHuman({
      editedSide: 'a',
      editedHuman: '1',
      pool: pool1m2m(),
      decimalsA: 6,
      decimalsB: 6,
      needsWrapA: true,
      needsWrapB: false,
      wrapMapperFeeBps: 0,
    })
    const withFee = computeProvideCounterpartHuman({
      editedSide: 'a',
      editedHuman: '1',
      pool: pool1m2m(),
      decimalsA: 6,
      decimalsB: 6,
      needsWrapA: true,
      needsWrapB: false,
      wrapMapperFeeBps: 100,
    })
    expect(feeFree).not.toBeNull()
    expect(withFee).not.toBeNull()
    expect(Number(withFee)).toBeLessThan(Number(feeFree))
  })
})
