import { describe, it, expect } from 'vitest'
import { findRoute, getAllTokens } from './router'
import type { PairInfo } from '@/types'

function mockPair(tokenA: string, tokenB: string, addr: string): PairInfo {
  return {
    asset_infos: [{ token: { contract_addr: tokenA } }, { token: { contract_addr: tokenB } }],
    contract_addr: addr,
    liquidity_token: `lp_${addr}`,
  }
}

describe('findRoute', () => {
  const pairs = [
    mockPair('tokenA', 'tokenB', 'pair1'),
    mockPair('tokenB', 'tokenC', 'pair2'),
    mockPair('tokenC', 'tokenD', 'pair3'),
  ]

  it('finds direct route', () => {
    const route = findRoute(pairs, 'tokenA', 'tokenB')
    expect(route).toHaveLength(1)
  })

  it('finds 2-hop route', () => {
    const route = findRoute(pairs, 'tokenA', 'tokenC')
    expect(route).toHaveLength(2)
  })

  it('finds 3-hop route', () => {
    const route = findRoute(pairs, 'tokenA', 'tokenD')
    expect(route).toHaveLength(3)
  })

  it('returns null for same token', () => {
    expect(findRoute(pairs, 'tokenA', 'tokenA')).toBeNull()
  })

  it('returns null for unreachable token', () => {
    expect(findRoute(pairs, 'tokenA', 'tokenZ')).toBeNull()
  })

  it('respects max 4 hops', () => {
    const longPairs = [
      mockPair('t1', 't2', 'p1'),
      mockPair('t2', 't3', 'p2'),
      mockPair('t3', 't4', 'p3'),
      mockPair('t4', 't5', 'p4'),
      mockPair('t5', 't6', 'p5'),
    ]
    const route = findRoute(longPairs, 't1', 't6')
    expect(route).toBeNull()
  })
})

describe('getAllTokens', () => {
  it('extracts unique tokens from pairs', () => {
    const pairs = [mockPair('tokenA', 'tokenB', 'pair1'), mockPair('tokenB', 'tokenC', 'pair2')]
    const tokens = getAllTokens(pairs)
    expect(tokens).toHaveLength(3)
    expect(tokens).toContain('tokenA')
    expect(tokens).toContain('tokenB')
    expect(tokens).toContain('tokenC')
  })
})
