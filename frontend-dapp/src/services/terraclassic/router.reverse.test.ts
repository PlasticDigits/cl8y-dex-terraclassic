import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { PairInfo } from '@/types'

const { CLUNC, CUSTC, taxParams } = vi.hoisted(() => ({
  CLUNC: 'terra1clunc_reverse_test',
  CUSTC: 'terra1custc_reverse_test',
  taxParams: { rate: '0.015', capUluna: 1_000_000_000_000_000n },
}))

vi.mock('@/utils/constants', async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ROUTER_CONTRACT_ADDRESS: 'terra1router_reverse_test',
  NATIVE_WRAPPED_PAIRS: { uluna: CLUNC, uusd: CUSTC },
}))
vi.mock('@/types', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/types')>()),
  getWrappedEquivalent: (id: string) => ({ uluna: CLUNC, uusd: CUSTC })[id] ?? null,
}))
vi.mock('./queries', () => ({ queryContract: vi.fn() }))
vi.mock('./transactions', () => ({ executeTerraContract: vi.fn(), executeTerraContractMulti: vi.fn() }))
vi.mock('./wrapMapper', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./wrapMapper')>()),
  queryWrapMapperFeeBps: vi.fn(),
}))
vi.mock('@/utils/nativeTransferTax', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/nativeTransferTax')>()
  return {
    ...actual,
    fetchNativeTransferTaxParams: vi.fn(async () => taxParams),
    netUlunaAfterTransferTaxAsync: vi.fn(async (gross: bigint) => actual.netUlunaAfterTransferTax(gross, taxParams)),
  }
})

import { queryContract } from './queries'
import { netAfterWrapMapperFee, queryWrapMapperFeeBps, wrapGrossForNetCw20 } from './wrapMapper'
import { fetchNativeTransferTaxParams } from '@/utils/nativeTransferTax'
import { reverseSimulateMultiHopSwap, reverseSimulateNativeSwap, simulateNativeSwap } from './router'

function pair(a: string, b: string): PairInfo {
  return {
    asset_infos: [{ token: { contract_addr: a } }, { token: { contract_addr: b } }],
    contract_addr: `pair_${a}_${b}`,
    liquidity_token: 'lp',
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  taxParams.rate = '0.015'
  taxParams.capUluna = 1_000_000_000_000_000n
  vi.mocked(queryWrapMapperFeeBps).mockImplementation(async (kind) => (kind === 'wrap' ? 200 : 51))
  vi.mocked(queryContract).mockImplementation(async (_address, message) => {
    const q = message as Record<string, { ask_amount?: string; offer_amount?: string }>
    if (q.reverse_simulate_swap_operations) {
      return { amount: (BigInt(q.reverse_simulate_swap_operations.ask_amount!) * 2n).toString() }
    }
    return { amount: (BigInt(q.simulate_swap_operations.offer_amount!) / 2n).toString() }
  })
})

describe('wrap reverse floor arithmetic (#1256 W8/W9)', () => {
  it.each([0, 1, 51, 100, 200, 9999])('finds minimal gross at %i bps, including raw dust', (bps) => {
    for (const target of [1n, 2n, 49n, 50n, 99n, 10000n, 990000n, 10n ** 40n]) {
      const gross = wrapGrossForNetCw20(target, bps)
      expect(netAfterWrapMapperFee(gross, bps)).toBe(target)
      expect(netAfterWrapMapperFee(gross - 1n, bps)).toBeLessThan(target)
    }
  })

  it.each([-1, 0.5, 10000, 10001, NaN, Infinity])('rejects impossible/invalid fee %s', (bps) => {
    expect(() => wrapGrossForNetCw20(1n, bps)).toThrow('Invalid wrap mapper fee')
  })
})

describe('reverseSimulateNativeSwap (#1256)', () => {
  it.each([
    ['uluna', CLUNC],
    ['uusd', CUSTC],
    [CLUNC, 'uluna'],
    [CUSTC, 'uusd'],
  ])('inverts direct %s → %s wallet receipt without router queries', async (from, to) => {
    for (const ask of ['1', '50', '9800', '9799765000']) {
      const reverse = await reverseSimulateNativeSwap(ask, from, to, [])
      const forward = await simulateNativeSwap(reverse.amount, from, to, [])
      expect(reverse.isDirectWrapUnwrap).toBe(true)
      expect(forward.amount).toBe(ask)
      if (to === 'uluna' || to === 'uusd') {
        expect(BigInt(forward.routerMinReceiveBase)).toBeGreaterThanOrEqual(BigInt(ask))
      } else {
        expect(fetchNativeTransferTaxParams).not.toHaveBeenCalled()
      }
    }
    expect(queryContract).not.toHaveBeenCalled()
  })

  it('accounts for capped native tax before mapper fee inversion', async () => {
    taxParams.capUluna = 100n
    const reverse = await reverseSimulateNativeSwap('1000000', CLUNC, 'uluna', [])
    const forward = await simulateNativeSwap(reverse.amount, CLUNC, 'uluna', [])
    expect(forward.amount).toBe('1000000')
    expect(forward.routerMinReceiveBase).toBe('1000100')
  })

  it.each([
    ['uluna', 'out', [pair(CLUNC, 'out')]],
    ['in', 'uluna', [pair('in', CLUNC)]],
    ['uluna', 'uusd', [pair(CLUNC, CUSTC)]],
    ['uluna', 'out', [pair(CLUNC, 'middle'), pair('middle', 'out')]],
  ])('sizes routed %s → %s with client operations and forward wallet agreement', async (from, to, pairs) => {
    const reverse = await reverseSimulateNativeSwap('9800', from, to, pairs, { trader: 'wallet', sender: 'sender' })
    const reverseQuery = vi.mocked(queryContract).mock.calls[0][1] as {
      reverse_simulate_swap_operations: { ask_amount: string; operations: unknown[]; trader: string; sender: string }
    }
    expect(reverse.isDirectWrapUnwrap).toBe(false)
    expect(reverseQuery.reverse_simulate_swap_operations.operations).toHaveLength(pairs.length)
    expect(reverseQuery.reverse_simulate_swap_operations.trader).toBe('wallet')
    expect(reverseQuery.reverse_simulate_swap_operations.sender).toBe('sender')
    if (to === 'uluna' || to === 'uusd') {
      expect(BigInt(reverseQuery.reverse_simulate_swap_operations.ask_amount)).toBeGreaterThan(9800n)
    }
    const forward = await simulateNativeSwap(reverse.amount, from, to, pairs, { trader: 'wallet', sender: 'sender' })
    expect(forward.amount).toBe('9800')
    expect(vi.mocked(queryContract).mock.calls[1][1]).toMatchObject({
      simulate_swap_operations: { trader: 'wallet', sender: 'sender' },
    })
    if (to === 'uluna' || to === 'uusd') {
      expect(BigInt(forward.routerMinReceiveBase)).toBeGreaterThan(9800n)
    }
  })

  it('rejects missing client operations without querying an exact-out solver', async () => {
    await expect(reverseSimulateNativeSwap('1', 'uluna', 'out', [])).rejects.toThrow('No route found')
    expect(queryContract).not.toHaveBeenCalled()
  })

  it.each(['', '0', '-1', '1.5', '<script>'])('rejects malformed or nonpositive raw receive %s', async (ask) => {
    await expect(reverseSimulateNativeSwap(ask, 'uluna', CLUNC, [])).rejects.toThrow('Invalid receive amount')
    expect(queryContract).not.toHaveBeenCalled()
  })

  it('fails closed on unavailable mapper config and invalid burn tax', async () => {
    vi.mocked(queryWrapMapperFeeBps).mockRejectedValueOnce(new Error('Wrap mapper config unavailable'))
    await expect(reverseSimulateNativeSwap('1', 'uluna', CLUNC, [])).rejects.toThrow('config unavailable')
    taxParams.rate = '1'
    await expect(reverseSimulateNativeSwap('1', CLUNC, 'uluna', [])).rejects.toThrow('Invalid native transfer tax')
  })

  it('rejects a zero offer from router reverse', async () => {
    vi.mocked(queryContract).mockResolvedValue({ amount: '0' })
    await expect(reverseSimulateNativeSwap('1', 'uluna', 'out', [pair(CLUNC, 'out')])).rejects.toThrow(
      'Invalid reverse swap offer'
    )
  })

  it('keeps the existing two-argument router reverse wire format', async () => {
    await reverseSimulateMultiHopSwap('5', [])
    expect(queryContract).toHaveBeenCalledWith('terra1router_reverse_test', {
      reverse_simulate_swap_operations: { ask_amount: '5', operations: [] },
    })
  })
})
