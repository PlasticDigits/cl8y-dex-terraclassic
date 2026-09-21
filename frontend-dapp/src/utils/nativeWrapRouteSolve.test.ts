import { describe, it, expect, vi } from 'vitest'

const { MOCK_LUNC_C, MOCK_USTC_C } = vi.hoisted(() => ({
  MOCK_LUNC_C: 'terra1lunc_c_mock_address_for_testing_xxxxx',
  MOCK_USTC_C: 'terra1ustc_c_mock_address_for_testing_xxxxx',
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    LUNC_C_TOKEN_ADDRESS: MOCK_LUNC_C,
    USTC_C_TOKEN_ADDRESS: MOCK_USTC_C,
    NATIVE_WRAPPED_PAIRS: { uluna: MOCK_LUNC_C, uusd: MOCK_USTC_C } as Record<string, string>,
    WRAPPED_NATIVE_PAIRS: { [MOCK_LUNC_C]: 'uluna', [MOCK_USTC_C]: 'uusd' } as Record<string, string>,
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
  }
})

vi.mock('@/services/terraclassic/router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/router')>()
  return {
    ...actual,
    isDirectWrapUnwrap: (from: string, to: string) => {
      if (from === 'uluna' && to === MOCK_LUNC_C) return 'wrap'
      if (from === MOCK_LUNC_C && to === 'uluna') return 'unwrap'
      if (from === 'uusd' && to === MOCK_USTC_C) return 'wrap'
      if (from === MOCK_USTC_C && to === 'uusd') return 'unwrap'
      return null
    },
  }
})

import { wrapMappedSolvePair, poolOnlyNativeExecuteOps } from './nativeWrapRouteSolve'
import type { SwapOperation } from '@/services/terraclassic/router'

const USTR = 'terra1ustr_mock_address_for_testing_xxxx'
const CL8Y = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

describe('wrapMappedSolvePair (#1218)', () => {
  it('maps native LUNC pay to cLUNC for USTR ask (P1)', () => {
    expect(wrapMappedSolvePair('uluna', USTR)).toEqual({
      tokenIn: MOCK_LUNC_C,
      tokenOut: USTR,
      needsWrapInput: true,
      needsUnwrapOutput: false,
    })
  })

  it('maps native USTC pay to cUSTC (P6)', () => {
    expect(wrapMappedSolvePair('uusd', USTR)).toEqual({
      tokenIn: MOCK_USTC_C,
      tokenOut: USTR,
      needsWrapInput: true,
      needsUnwrapOutput: false,
    })
  })

  it('maps unwrap-exit USTR → LUNC to USTR → cLUNC (P5)', () => {
    expect(wrapMappedSolvePair(USTR, 'uluna')).toEqual({
      tokenIn: USTR,
      tokenOut: MOCK_LUNC_C,
      needsWrapInput: false,
      needsUnwrapOutput: true,
    })
  })

  it('maps LUNC → listed CL8Y wrap-enter (P7 / AC6)', () => {
    expect(wrapMappedSolvePair('uluna', CL8Y)).toEqual({
      tokenIn: MOCK_LUNC_C,
      tokenOut: CL8Y,
      needsWrapInput: true,
      needsUnwrapOutput: false,
    })
  })

  it('does not map direct 1:1 wrap or unwrap (P4 / AC4)', () => {
    expect(wrapMappedSolvePair('uluna', MOCK_LUNC_C)).toBeNull()
    expect(wrapMappedSolvePair(MOCK_LUNC_C, 'uluna')).toBeNull()
    expect(wrapMappedSolvePair('uusd', MOCK_USTC_C)).toBeNull()
  })

  it('does not map CW20↔CW20 (P10)', () => {
    expect(wrapMappedSolvePair(MOCK_LUNC_C, USTR)).toBeNull()
  })
})

describe('poolOnlyNativeExecuteOps (#1218 P3 / H596-7)', () => {
  it('strips hybrid / book_input on every hop', () => {
    const ops: SwapOperation[] = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: MOCK_LUNC_C } },
          ask_asset_info: { token: { contract_addr: USTR } },
          hybrid: { pool_input: '1', book_input: '2', max_maker_fills: 8 },
        },
      },
    ]
    const stripped = poolOnlyNativeExecuteOps(ops)
    expect(stripped[0].terra_swap.hybrid).toBeUndefined()
    expect(stripped[0].terra_swap.offer_asset_info).toEqual(ops[0].terra_swap.offer_asset_info)
  })
})
