import { describe, it, expect } from 'vitest'
import { resolveSwapRoutePairAddresses } from '../resolveSwapRoutePairAddresses'
import type { PairInfo } from '@/types'
import type { SwapOperation } from '@/services/terraclassic/router'

const TOKEN_A = 'terra1tokenaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const TOKEN_B = 'terra1tokenbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
const TOKEN_C = 'terra1tokencccccccccccccccccccccccccccccc'
const PAIR_AB = 'terra1pairabababababababababababababababab'
const PAIR_BC = 'terra1pairbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbc'

function pair(addr: string, t0: string, t1: string): PairInfo {
  return {
    contract_addr: addr,
    liquidity_token: `${addr}-lp`,
    asset_infos: [{ token: { contract_addr: t0 } }, { token: { contract_addr: t1 } }],
  }
}

function op(offer: string, ask: string): SwapOperation {
  return {
    terra_swap: {
      offer_asset_info: { token: { contract_addr: offer } },
      ask_asset_info: { token: { contract_addr: ask } },
    },
  }
}

describe('resolveSwapRoutePairAddresses (#449)', () => {
  const pairs = [pair(PAIR_AB, TOKEN_A, TOKEN_B), pair(PAIR_BC, TOKEN_B, TOKEN_C)]

  it('returns direct pair address when route ops are absent', () => {
    expect(
      resolveSwapRoutePairAddresses({
        routeOps: null,
        pairs,
        directPair: pairs[0],
        fromToken: TOKEN_A,
        toToken: TOKEN_B,
      })
    ).toEqual([PAIR_AB])
  })

  it('resolves factory pair per hop from route operations', () => {
    const routeOps: SwapOperation[] = [op(TOKEN_A, TOKEN_B), op(TOKEN_B, TOKEN_C)]
    expect(
      resolveSwapRoutePairAddresses({
        routeOps,
        pairs,
        directPair: null,
        fromToken: TOKEN_A,
        toToken: TOKEN_C,
      })
    ).toEqual([PAIR_AB, PAIR_BC])
  })

  it('falls back to from/to token lookup when ops and direct pair are missing', () => {
    expect(
      resolveSwapRoutePairAddresses({
        routeOps: [],
        pairs,
        directPair: null,
        fromToken: TOKEN_B,
        toToken: TOKEN_C,
      })
    ).toEqual([PAIR_BC])
  })

  it('returns empty when no factory match exists', () => {
    expect(
      resolveSwapRoutePairAddresses({
        routeOps: [op(TOKEN_A, TOKEN_C)],
        pairs,
        directPair: null,
        fromToken: TOKEN_A,
        toToken: TOKEN_C,
      })
    ).toEqual([])
  })
})
