import { describe, it, expect, vi } from 'vitest'

const { MOCK_LUNC_C, MOCK_USTC_C } = vi.hoisted(() => ({
  MOCK_LUNC_C: 'terra1lunc_c_mock_address_for_testing_xxxxx',
  MOCK_USTC_C: 'terra1ustc_c_mock_address_for_testing_xxxxx',
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>()
  return {
    ...actual,
    ROUTER_CONTRACT_ADDRESS: 'terra1router_mock',
    TREASURY_CONTRACT_ADDRESS: 'terra1treasury_mock',
    WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1wrap_mapper_mock',
    LUNC_C_TOKEN_ADDRESS: MOCK_LUNC_C,
    USTC_C_TOKEN_ADDRESS: MOCK_USTC_C,
    NATIVE_WRAPPED_PAIRS: {
      uluna: MOCK_LUNC_C,
      uusd: MOCK_USTC_C,
    } as Record<string, string>,
    WRAPPED_NATIVE_PAIRS: {
      [MOCK_LUNC_C]: 'uluna',
      [MOCK_USTC_C]: 'uusd',
    } as Record<string, string>,
  }
})

vi.mock('@/types', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/types')>()
  return {
    ...actual,
    getWrappedEquivalent: (tokenId: string) => {
      const map: Record<string, string> = { uluna: MOCK_LUNC_C, uusd: MOCK_USTC_C }
      return map[tokenId] ?? null
    },
    getNativeEquivalent: (tokenId: string) => {
      const map: Record<string, string> = { [MOCK_LUNC_C]: 'uluna', [MOCK_USTC_C]: 'uusd' }
      return map[tokenId] ?? null
    },
  }
})

import { findRoute, getAllTokens, isDirectWrapUnwrap, findRouteWithNativeSupport } from './router'
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

describe('isDirectWrapUnwrap', () => {
  it('returns null for unrelated CW20 tokens', () => {
    expect(isDirectWrapUnwrap('terra1abc', 'terra1def')).toBeNull()
  })

  it('returns null for same token', () => {
    expect(isDirectWrapUnwrap('uluna', 'uluna')).toBeNull()
  })

  it('returns wrap for uluna -> LUNC-C', () => {
    expect(isDirectWrapUnwrap('uluna', MOCK_LUNC_C)).toBe('wrap')
  })

  it('returns unwrap for LUNC-C -> uluna', () => {
    expect(isDirectWrapUnwrap(MOCK_LUNC_C, 'uluna')).toBe('unwrap')
  })

  it('returns wrap for uusd -> USTC-C', () => {
    expect(isDirectWrapUnwrap('uusd', MOCK_USTC_C)).toBe('wrap')
  })

  it('returns unwrap for USTC-C -> uusd', () => {
    expect(isDirectWrapUnwrap(MOCK_USTC_C, 'uusd')).toBe('unwrap')
  })
})

describe('findRouteWithNativeSupport', () => {
  it('returns null for direct wrap/unwrap (uluna -> LUNC-C)', () => {
    const pairs = [mockPair(MOCK_LUNC_C, 'tokenB', 'pair1')]
    expect(findRouteWithNativeSupport(pairs, 'uluna', MOCK_LUNC_C)).toBeNull()
  })

  it('returns route with needsWrapInput when from-token is native', () => {
    const pairs = [mockPair(MOCK_LUNC_C, 'tokenB', 'pair1')]
    const result = findRouteWithNativeSupport(pairs, 'uluna', 'tokenB')
    expect(result).not.toBeNull()
    expect(result!.needsWrapInput).toBe(true)
    expect(result!.needsUnwrapOutput).toBe(false)
    expect(result!.operations).toHaveLength(1)
  })

  it('returns route with needsUnwrapOutput when to-token is native', () => {
    const pairs = [mockPair('tokenA', MOCK_LUNC_C, 'pair1')]
    const result = findRouteWithNativeSupport(pairs, 'tokenA', 'uluna')
    expect(result).not.toBeNull()
    expect(result!.needsWrapInput).toBe(false)
    expect(result!.needsUnwrapOutput).toBe(true)
    expect(result!.operations).toHaveLength(1)
  })

  it('returns route with both wrap and unwrap for native-to-native', () => {
    const pairs = [mockPair(MOCK_LUNC_C, MOCK_USTC_C, 'pair1')]
    const result = findRouteWithNativeSupport(pairs, 'uluna', 'uusd')
    expect(result).not.toBeNull()
    expect(result!.needsWrapInput).toBe(true)
    expect(result!.needsUnwrapOutput).toBe(true)
  })
})

describe('getAllTokens with native support', () => {
  it('returns CW20 tokens when no native mapping configured', () => {
    const pairs = [mockPair('tokenA', 'tokenB', 'pair1')]
    const tokens = getAllTokens(pairs)
    expect(tokens).toContain('tokenA')
    expect(tokens).toContain('tokenB')
  })

  it('includes native denoms when wrapped equivalent is in pair graph', () => {
    const pairs = [mockPair(MOCK_LUNC_C, 'tokenB', 'pair1')]
    const tokens = getAllTokens(pairs)
    expect(tokens).toContain(MOCK_LUNC_C)
    expect(tokens).toContain('tokenB')
    expect(tokens).toContain('uluna')
  })

  it('includes both native denoms when both wrapped tokens exist', () => {
    const pairs = [mockPair(MOCK_LUNC_C, 'tokenB', 'pair1'), mockPair(MOCK_USTC_C, 'tokenC', 'pair2')]
    const tokens = getAllTokens(pairs)
    expect(tokens).toContain('uluna')
    expect(tokens).toContain('uusd')
  })
})
