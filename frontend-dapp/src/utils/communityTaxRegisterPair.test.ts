import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  compareUnregisteredPairLp,
  isCommunityTaxCodeId,
  otherManagedTokensNeedingRegister,
  pickHighestLpUnregistered,
  registerLargestPoolLabel,
  registerTaxAssetsAfterCreatePair,
  shortPairAddr,
  type UnregisteredFactoryPair,
} from './communityTaxRegisterPair'

const PAIR_A = 'terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const PAIR_B = 'terra1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
const TAX = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const HONEST = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

vi.mock('@/services/indexer/client', () => ({
  getCommunityTokens: vi.fn(),
  getHubPrices: vi.fn(),
  getTokenPairs: vi.fn(),
}))

vi.mock('@/services/terraclassic/factory', () => ({
  getPair: vi.fn(),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  getPairInfo: vi.fn(),
  getPool: vi.fn(),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  getChainContractInfo: vi.fn(),
}))

vi.mock('@/services/terraclassic/communityTaxToken', () => ({
  queryCommunityTaxIsExempt: vi.fn(),
  registerListedPair: vi.fn(),
}))

function pair(over: Partial<UnregisteredFactoryPair> & { pair: string }): UnregisteredFactoryPair {
  return {
    symbols: ['TAX', 'UST1'],
    usdTvl: null,
    taxReserve: 0n,
    otherReserve: 0n,
    ...over,
  }
}

describe('communityTaxRegisterPair (#633)', () => {
  it('isCommunityTaxCodeId matches the pin only', () => {
    expect(isCommunityTaxCodeId(11619, 11619)).toBe(true)
    expect(isCommunityTaxCodeId(10184, 11619)).toBe(false)
    expect(isCommunityTaxCodeId(11619, 0)).toBe(false)
  })

  it('highest LP prefers USD when both sides price, else tax reserve, then other, then addr', () => {
    const lowUsd = pair({ pair: PAIR_A, usdTvl: 10, taxReserve: 9_000n })
    const highUsd = pair({ pair: PAIR_B, usdTvl: 50, taxReserve: 1n })
    expect(pickHighestLpUnregistered([lowUsd, highUsd])?.pair).toBe(PAIR_B)

    const lowTax = pair({ pair: PAIR_A, taxReserve: 100n, otherReserve: 9_000n })
    const highTax = pair({ pair: PAIR_B, taxReserve: 500n, otherReserve: 1n })
    expect(pickHighestLpUnregistered([lowTax, highTax])?.pair).toBe(PAIR_B)

    const lowOther = pair({ pair: PAIR_A, taxReserve: 10n, otherReserve: 1n })
    const highOther = pair({ pair: PAIR_B, taxReserve: 10n, otherReserve: 80n })
    expect(pickHighestLpUnregistered([lowOther, highOther])?.pair).toBe(PAIR_B)

    const a = pair({ pair: PAIR_A, taxReserve: 10n, otherReserve: 10n })
    const b = pair({ pair: PAIR_B, taxReserve: 10n, otherReserve: 10n })
    expect(pickHighestLpUnregistered([b, a])?.pair).toBe(PAIR_A)
  })

  it('compare is factory-only input — caller drops non-factory', () => {
    expect(compareUnregisteredPairLp(pair({ pair: PAIR_B }), pair({ pair: PAIR_A }))).toBeGreaterThan(0)
  })

  it('button label uses symbols + truncated addr, not execute names', () => {
    const label = registerLargestPoolLabel(pair({ pair: PAIR_A, symbols: ['EMBER', 'QATax'] }))
    expect(label).toMatch(/largest pool/)
    expect(label).toMatch(/EMBER\/QATax/)
    expect(label).not.toMatch(/RegisterListedPair|LISTED_PAIRS|VITE_/)
    expect(shortPairAddr(PAIR_A)).toMatch(/…/)
  })

  it('cross-token list excludes current and uses attested catalog rows only', () => {
    const rows = otherManagedTokensNeedingRegister(
      TAX,
      [
        { contract_address: TAX, symbol: 'CUR', attested_cmm: true } as never,
        { contract_address: HONEST, symbol: 'OTH', attested_cmm: true } as never,
      ],
      new Set([HONEST.toLowerCase(), 'terra1rogue'])
    )
    expect(rows).toEqual([{ address: HONEST, symbol: 'OTH' }])
  })
})

describe('registerTaxAssetsAfterCreatePair (#633 B1)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('registers only tax-pin assets', async () => {
    const { getPair } = await import('@/services/terraclassic/factory')
    const { getChainContractInfo } = await import('@/services/terraclassic/queries')
    const { registerListedPair } = await import('@/services/terraclassic/communityTaxToken')
    vi.mocked(getPair).mockResolvedValue({
      contract_addr: PAIR_A,
      asset_infos: [{ token: { contract_addr: TAX } }, { token: { contract_addr: HONEST } }],
      liquidity_token: 'lp',
    })
    vi.mocked(getChainContractInfo).mockImplementation(async (addr: string) => ({
      code_id: addr === TAX ? 20 : 10,
      admin: '',
      creator: '',
      label: '',
    }))
    vi.mocked(registerListedPair).mockResolvedValue('reg-tx')

    const r = await registerTaxAssetsAfterCreatePair({
      wallet: 'terra1wallet',
      tokenA: TAX,
      tokenB: HONEST,
      taxCodeId: 20,
    })
    expect(r.pair).toBe(PAIR_A)
    expect(r.registered).toEqual([TAX])
    expect(registerListedPair).toHaveBeenCalledTimes(1)
    expect(registerListedPair).toHaveBeenCalledWith('terra1wallet', TAX, PAIR_A)
  })

  it('honest/honest does not call register', async () => {
    const { getPair } = await import('@/services/terraclassic/factory')
    const { getChainContractInfo } = await import('@/services/terraclassic/queries')
    const { registerListedPair } = await import('@/services/terraclassic/communityTaxToken')
    vi.mocked(getPair).mockResolvedValue({
      contract_addr: PAIR_B,
      asset_infos: [{ token: { contract_addr: HONEST } }, { token: { contract_addr: TAX } }],
      liquidity_token: 'lp',
    })
    vi.mocked(getChainContractInfo).mockResolvedValue({ code_id: 10, admin: '', creator: '', label: '' })
    const r = await registerTaxAssetsAfterCreatePair({
      wallet: 'terra1wallet',
      tokenA: HONEST,
      tokenB: TAX,
      taxCodeId: 20,
    })
    expect(r.registered).toEqual([])
    expect(registerListedPair).not.toHaveBeenCalled()
  })

  it('tax/tax registers both and surfaces a hard error if one fails', async () => {
    const { getPair } = await import('@/services/terraclassic/factory')
    const { getChainContractInfo } = await import('@/services/terraclassic/queries')
    const { registerListedPair } = await import('@/services/terraclassic/communityTaxToken')
    vi.mocked(getPair).mockResolvedValue({
      contract_addr: PAIR_A,
      asset_infos: [{ token: { contract_addr: TAX } }, { token: { contract_addr: HONEST } }],
      liquidity_token: 'lp',
    })
    vi.mocked(getChainContractInfo).mockResolvedValue({ code_id: 20, admin: '', creator: '', label: '' })
    vi.mocked(registerListedPair).mockResolvedValueOnce('ok').mockRejectedValueOnce(new Error('lcd down'))

    await expect(
      registerTaxAssetsAfterCreatePair({
        wallet: 'terra1wallet',
        tokenA: TAX,
        tokenB: HONEST,
        taxCodeId: 20,
      })
    ).rejects.toThrow(/Open Manage/)
    expect(registerListedPair).toHaveBeenCalledTimes(2)
  })
})
