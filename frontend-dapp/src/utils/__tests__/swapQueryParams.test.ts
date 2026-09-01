import { describe, expect, it } from 'vitest'
import { COLUMBUS5_GEM_ADDRESSES, isGemTokenId } from '@/utils/pairCatalogRank'
import {
  applySwapQueryParams,
  canonicalSwapSearch,
  parseSwapExactField,
  parseSwapQueryParams,
  resolveSwapQueryTokenValue,
  swapDeepLinkPath,
} from '@/utils/swapQueryParams'
import { lookupTokenIdByProductTicker } from '@/utils/tokenRegistry'
import {
  MAINNET_CUSTC_TOKEN_ADDRESS,
  MAINNET_UST1_TOKEN_ADDRESS,
  MAINNET_VFDUSD_TOKEN_ADDRESS,
} from '@/utils/ust1SecondaryMarket'

const CL8Y = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const USTR = 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv'
const CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const LISTED = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const RUBY = 'terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc'
const DEFAULTS = ['uluna', 'uusd'] as const
const FACTORY = [
  'uluna',
  'uusd',
  CL8Y,
  USTR,
  MAINNET_UST1_TOKEN_ADDRESS,
  CLUNC,
  MAINNET_CUSTC_TOKEN_ADDRESS,
  LISTED,
  RUBY,
]

function apply(
  search: string,
  opts?: { isHiddenToken?: (id: string) => boolean; factory?: readonly string[]; defaults?: readonly [string, string] }
) {
  return applySwapQueryParams(search, opts?.factory ?? FACTORY, opts?.defaults ?? DEFAULTS, {
    isHiddenToken: opts?.isHiddenToken,
  })
}

describe('resolveSwapQueryTokenValue (#711)', () => {
  it('maps native denoms and product tickers', () => {
    expect(resolveSwapQueryTokenValue('uluna')).toBe('uluna')
    expect(resolveSwapQueryTokenValue('ULUNA')).toBe('uluna')
    expect(resolveSwapQueryTokenValue('LUNC')).toBe('uluna')
    expect(resolveSwapQueryTokenValue('lunc')).toBe('uluna')
    expect(resolveSwapQueryTokenValue('uusd')).toBe('uusd')
    expect(resolveSwapQueryTokenValue('USTC')).toBe('uusd')
    expect(resolveSwapQueryTokenValue('UST')).toBe('uusd')
    expect(resolveSwapQueryTokenValue('CL8Y')).toBe(CL8Y)
    expect(resolveSwapQueryTokenValue('UST1')).toBe(lookupTokenIdByProductTicker('UST1'))
    expect(resolveSwapQueryTokenValue('cLUNC')).toBe(lookupTokenIdByProductTicker('cLUNC'))
    expect(resolveSwapQueryTokenValue('cUSTC')).toBe(lookupTokenIdByProductTicker('cUSTC'))
    expect(resolveSwapQueryTokenValue('vFDUSD')).toBe(lookupTokenIdByProductTicker('vFDUSD'))
    expect(resolveSwapQueryTokenValue('USTR')).toBe(USTR)
    expect(resolveSwapQueryTokenValue('SpaceUSD')).toBe(
      'terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl'
    )
    expect(resolveSwapQueryTokenValue('spaceusd')).toBe(
      'terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl'
    )
    expect(resolveSwapQueryTokenValue(MAINNET_UST1_TOKEN_ADDRESS)).toBe(MAINNET_UST1_TOKEN_ADDRESS)
  })

  it('accepts checksummed terra1 and rejects format/checksum failures', () => {
    expect(resolveSwapQueryTokenValue(LISTED)).toBe(LISTED)
    expect(resolveSwapQueryTokenValue('terra1')).toBeNull()
    expect(resolveSwapQueryTokenValue(`${LISTED.slice(0, -3)}289`)).toBeNull()
  })

  it('ignores hostile, EVM, IBC, and overlong values (A1–A4 / A12 / A14)', () => {
    expect(resolveSwapQueryTokenValue('javascript:alert(1)')).toBeNull()
    expect(resolveSwapQueryTokenValue('data:text/html,x')).toBeNull()
    expect(resolveSwapQueryTokenValue('https://evil')).toBeNull()
    expect(resolveSwapQueryTokenValue('//evil')).toBeNull()
    expect(resolveSwapQueryTokenValue('<img onerror=1>')).toBeNull()
    expect(resolveSwapQueryTokenValue('%3Cscript%3E')).toBeNull()
    expect(resolveSwapQueryTokenValue('0xabc123')).toBeNull()
    expect(resolveSwapQueryTokenValue('ETH')).toBeNull()
    expect(resolveSwapQueryTokenValue('BNB')).toBeNull()
    expect(resolveSwapQueryTokenValue('WETH')).toBeNull()
    expect(resolveSwapQueryTokenValue('ibc/ABC')).toBeNull()
    expect(resolveSwapQueryTokenValue('factory/terra1/x')).toBeNull()
    expect(resolveSwapQueryTokenValue('x'.repeat(4000))).toBeNull()
    expect(resolveSwapQueryTokenValue('RUBY')).toBeNull()
    expect(resolveSwapQueryTokenValue('')).toBeNull()
  })
})

describe('parseSwapQueryParams (#711)', () => {
  it('prefers Uniswap family over Terra when both are set', () => {
    expect(parseSwapQueryParams('from=uluna&inputCurrency=uusd')).toMatchObject({
      payId: 'uusd',
      receiveId: null,
    })
    expect(parseSwapQueryParams('to=uluna&outputCurrency=uusd')).toMatchObject({
      payId: null,
      receiveId: 'uusd',
    })
  })

  it('uses last repeated key within a family (A13)', () => {
    expect(parseSwapQueryParams('from=uluna&from=uusd').payId).toBe('uusd')
    expect(parseSwapQueryParams('from=&from=uluna').payId).toBe('uluna')
  })

  it('matches alias names case-insensitively', () => {
    expect(parseSwapQueryParams('InputCurrency=uluna&OUTPUTCURRENCY=uusd')).toEqual({
      payId: 'uluna',
      receiveId: 'uusd',
      payAmountHuman: null,
      exactField: null,
    })
  })

  it('parses conservative pay amounts and rejects junk (A9)', () => {
    expect(parseSwapQueryParams('exactAmount=1.5').payAmountHuman).toBe('1.5')
    expect(parseSwapQueryParams('amount=1.5').payAmountHuman).toBe('1.5')
    expect(parseSwapQueryParams('value=2').payAmountHuman).toBe('2')
    expect(parseSwapQueryParams('amountIn=3').payAmountHuman).toBe('3')
    expect(parseSwapQueryParams('exactAmount=1e18').payAmountHuman).toBeNull()
    expect(parseSwapQueryParams('exactAmount=-1').payAmountHuman).toBeNull()
    expect(parseSwapQueryParams('exactAmount=1,5').payAmountHuman).toBeNull()
    expect(parseSwapQueryParams(`exactAmount=${'9'.repeat(40)}`).payAmountHuman).toBeNull()
    expect(parseSwapQueryParams('exactAmount=0').payAmountHuman).toBeNull()
  })

  it('does not read slippage, expertMode, recipient, hybrid, or showGems', () => {
    const parsed = parseSwapQueryParams(
      'showGems=1&expertMode=1&slippage=50&recipient=terra1x&toAddress=terra1x&pool_only=1&hybrid_optimize=0&from=uluna'
    )
    expect(parsed.payId).toBe('uluna')
    expect(parsed.receiveId).toBeNull()
    expect(parsed.payAmountHuman).toBeNull()
  })
})

describe('applySwapQueryParams (#711)', () => {
  it('T1: empty search keeps defaults', () => {
    expect(apply('')).toEqual({ payId: 'uluna', receiveId: 'uusd', payAmountHuman: null })
  })

  it('T2: from/to listed pair', () => {
    expect(apply(`from=uluna&to=${LISTED}`)).toEqual({
      payId: 'uluna',
      receiveId: LISTED,
      payAmountHuman: null,
    })
  })

  it('T3: Uniswap inputCurrency / outputCurrency', () => {
    expect(apply(`inputCurrency=uusd&outputCurrency=${LISTED}`)).toEqual({
      payId: 'uusd',
      receiveId: LISTED,
      payAmountHuman: null,
    })
  })

  it('T4: outputCurrency only defaults pay to the other economic id', () => {
    expect(apply(`outputCurrency=${LISTED}`)).toEqual({
      payId: 'uluna',
      receiveId: LISTED,
      payAmountHuman: null,
    })
    expect(apply('from=uusd')).toEqual({
      payId: 'uusd',
      receiveId: 'uluna',
      payAmountHuman: null,
    })
  })

  it('T6: LUNC / UST1 tickers', () => {
    const ust1 = lookupTokenIdByProductTicker('UST1') ?? MAINNET_UST1_TOKEN_ADDRESS
    expect(apply('from=LUNC&to=UST1', { factory: [...FACTORY, ust1] })).toEqual({
      payId: 'uluna',
      receiveId: ust1,
      payAmountHuman: null,
    })
  })

  it('T7: same id both sides keeps pay and defaults receive', () => {
    expect(apply('from=uluna&to=uluna')).toEqual({
      payId: 'uluna',
      receiveId: 'uusd',
      payAmountHuman: null,
    })
  })

  it('T8: exactAmount prefills pay amount', () => {
    expect(apply(`from=uluna&to=${LISTED}&exactAmount=1.5`).payAmountHuman).toBe('1.5')
  })

  it('T11: wrap pair uluna → cLUNC', () => {
    const clunc = lookupTokenIdByProductTicker('cLUNC') ?? CLUNC
    expect(apply(`from=uluna&to=${clunc}`, { factory: [...FACTORY, clunc] })).toMatchObject({
      payId: 'uluna',
      receiveId: clunc,
    })
  })

  it('A5 / AC8: production gem hide ignores gem address and ticker', () => {
    expect(COLUMBUS5_GEM_ADDRESSES.has(RUBY)).toBe(true)
    expect(apply(`from=${RUBY}&to=${LISTED}`, { isHiddenToken: isGemTokenId })).toEqual({
      payId: 'uluna',
      receiveId: LISTED,
      payAmountHuman: null,
    })
    expect(apply('from=RUBY&to=uluna', { isHiddenToken: isGemTokenId })).toEqual({
      payId: 'uusd',
      receiveId: 'uluna',
      payAmountHuman: null,
    })
  })

  it('A11: unlisted but valid terra1 is ignored', () => {
    expect(apply(`from=uluna&to=${LISTED}`, { factory: ['uluna', 'uusd'] })).toEqual({
      payId: 'uluna',
      receiveId: 'uusd',
      payAmountHuman: null,
    })
  })

  it('does not honor showGems as an apply bypass', () => {
    expect(apply(`from=${RUBY}&showGems=1`, { isHiddenToken: isGemTokenId })).toEqual({
      payId: 'uluna',
      receiveId: 'uusd',
      payAmountHuman: null,
    })
  })
})

describe('canonicalSwapSearch / exactField (#713 / #715)', () => {
  it('writes tokenlist symbols when unique; omits empty amount; exactField=output only when output', () => {
    expect(canonicalSwapSearch({ payId: 'uluna', receiveId: 'uusd' }).toString()).toBe('from=LUNC&to=USTC')
    expect(canonicalSwapSearch({ payId: 'uluna', receiveId: 'uusd', amountHuman: '1.5' }).toString()).toBe(
      'from=LUNC&to=USTC&exactAmount=1.5'
    )
    expect(
      canonicalSwapSearch({ payId: 'uluna', receiveId: 'uusd', amountHuman: '', exactField: 'output' }).toString()
    ).toBe('from=LUNC&to=USTC&exactField=output')
    expect(canonicalSwapSearch({ payId: 'uluna', receiveId: 'uusd', amountHuman: '1e18' }).toString()).toBe(
      'from=LUNC&to=USTC'
    )
    expect(canonicalSwapSearch({ payId: 'uluna', receiveId: 'uusd', amountHuman: '9'.repeat(40) }).toString()).toBe(
      'from=LUNC&to=USTC'
    )
  })

  it('parses exactField=output and independentField=output; ignores input / junk', () => {
    expect(parseSwapExactField('exactField=output')).toBe('output')
    expect(parseSwapExactField('independentField=output')).toBe('output')
    expect(parseSwapQueryParams('exactField=output&exactAmount=1').exactField).toBe('output')
    expect(parseSwapExactField('exactField=input')).toBeNull()
    expect(parseSwapExactField('exactField=pay')).toBeNull()
    expect(parseSwapExactField('exactField=<script>')).toBeNull()
  })
})

describe('swapDeepLinkPath (#711 / #713 / #715)', () => {
  it('builds from/to via URLSearchParams with tokenlist symbols when unique', () => {
    expect(swapDeepLinkPath(MAINNET_UST1_TOKEN_ADDRESS, MAINNET_VFDUSD_TOKEN_ADDRESS)).toBe('/?from=UST1&to=vFDUSD')
    expect(swapDeepLinkPath('uluna', 'uusd', '1.5')).toBe('/?from=LUNC&to=USTC&exactAmount=1.5')
    expect(swapDeepLinkPath('uluna', 'uusd', '1e18')).toBe('/?from=LUNC&to=USTC')
    expect(swapDeepLinkPath('uluna', 'uusd', '1', 'output')).toBe('/?from=LUNC&to=USTC&exactAmount=1&exactField=output')
  })

  it('keeps unlisted factory bech32 (no invented ticker)', () => {
    expect(swapDeepLinkPath('uluna', LISTED)).toBe(`/?from=LUNC&to=${encodeURIComponent(LISTED)}`)
  })
})
