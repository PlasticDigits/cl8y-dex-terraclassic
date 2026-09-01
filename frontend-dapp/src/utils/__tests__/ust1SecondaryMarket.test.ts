import { describe, expect, it } from 'vitest'
import {
  assertSecondaryMarketCopy,
  copyImpliesAmmIsMintRedeem,
  isUst1SecondaryPairConfigured,
  MAINNET_CUSTC_TOKEN_ADDRESS,
  MAINNET_UST1_TOKEN_ADDRESS,
  MAINNET_VFDUSD_TOKEN_ADDRESS,
  resolvesUst1SecondaryTokens,
  UST1_CREATE_PAIR_SECONDARY_NOTICE,
  UST1_SECONDARY_MARKET_BLURB,
  ust1SecondarySwapPath,
  ust1SecondaryTradePath,
} from '../ust1SecondaryMarket'

describe('ust1SecondaryMarket (#508)', () => {
  it('resolves published mainnet anchors when env is empty', () => {
    const tokens = resolvesUst1SecondaryTokens('', '')
    expect(tokens).toEqual({
      ust1: MAINNET_UST1_TOKEN_ADDRESS,
      quote: MAINNET_VFDUSD_TOKEN_ADDRESS,
      quoteLeg: 'vFDUSD',
    })
    expect(resolvesUst1SecondaryTokens('', '', 'cUSTC')?.quote).toBe(MAINNET_CUSTC_TOKEN_ADDRESS)
  })

  it('builds Trade/Swap paths matching real routes (Swap query from/to, #711)', () => {
    expect(ust1SecondaryTradePath()).toBe('/trade')
    expect(ust1SecondaryTradePath('terra1pairaddrxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx')).toBe(
      '/trade/terra1pairaddrxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'
    )
    const swapPath = ust1SecondarySwapPath()
    expect(swapPath.startsWith('/?')).toBe(true)
    const q = new URLSearchParams(swapPath.slice(1))
    expect(q.get('from')).toBe('UST1')
    expect(q.get('to')).toBe('vFDUSD')
    expect(q.get('from')).not.toMatch(/^terra1/)
    expect(q.get('to')).not.toMatch(/^terra1/)
    expect(isUst1SecondaryPairConfigured()).toBe(false)
  })

  it('keeps retail copy from implying AMM is mint/redeem (U1)', () => {
    expect(copyImpliesAmmIsMintRedeem(UST1_SECONDARY_MARKET_BLURB)).toBe(false)
    expect(copyImpliesAmmIsMintRedeem(UST1_CREATE_PAIR_SECONDARY_NOTICE)).toBe(false)
    expect(() => assertSecondaryMarketCopy(UST1_CREATE_PAIR_SECONDARY_NOTICE)).not.toThrow()
    expect(copyImpliesAmmIsMintRedeem('Mint on the AMM to get UST1')).toBe(true)
    expect(copyImpliesAmmIsMintRedeem('Swap to mint UST1')).toBe(true)
  })
})
