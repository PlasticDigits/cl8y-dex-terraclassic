import { describe, expect, it } from 'vitest'
import {
  communityTaxViteEnvForPlaywright,
  isColumbus5CommunityTaxCodeId,
  MAINNET_COMMUNITY_TAX_CODE_ID,
  MAINNET_COMMUNITY_TOKEN_LAUNCHER,
  parseCommunityTaxTxPins,
} from './communityTaxTxEnv'

const LOCAL = {
  VITE_TOKEN_COMMUNITY_TAX_ADDRESS: 'terra1qataxtokenaddress00000000000000000000001',
  VITE_PAIR_COMMUNITY_TAX_EMBER: 'terra1qataxpairaddress000000000000000000000002',
  VITE_TOKEN_EMBER_ADDRESS: 'terra1emberaddress000000000000000000000000003',
  VITE_COMMUNITY_TOKEN_LAUNCHER: 'terra1locallauncher00000000000000000000000004',
  VITE_COMMUNITY_TAX_CODE_ID: '42',
}

describe('communityTaxTxEnv (#622)', () => {
  it('accepts local seed pins', () => {
    expect(parseCommunityTaxTxPins(LOCAL)).toEqual({
      token: LOCAL.VITE_TOKEN_COMMUNITY_TAX_ADDRESS,
      pair: LOCAL.VITE_PAIR_COMMUNITY_TAX_EMBER,
      ember: LOCAL.VITE_TOKEN_EMBER_ADDRESS,
      launcher: LOCAL.VITE_COMMUNITY_TOKEN_LAUNCHER,
      codeId: '42',
      symbol: 'QTAX',
    })
  })

  it('fails closed when tax pair pins are missing', () => {
    expect(() => parseCommunityTaxTxPins({})).toThrow(/VITE_TOKEN_COMMUNITY_TAX_ADDRESS/)
  })

  it('rejects columbus-5 code ids and launcher on the tx path', () => {
    expect(isColumbus5CommunityTaxCodeId('11611')).toBe(true)
    expect(isColumbus5CommunityTaxCodeId('11619')).toBe(true)
    expect(isColumbus5CommunityTaxCodeId('42')).toBe(false)
    expect(() =>
      parseCommunityTaxTxPins({ ...LOCAL, VITE_COMMUNITY_TAX_CODE_ID: MAINNET_COMMUNITY_TAX_CODE_ID })
    ).toThrow(/columbus-5/)
    expect(() =>
      parseCommunityTaxTxPins({ ...LOCAL, VITE_COMMUNITY_TOKEN_LAUNCHER: MAINNET_COMMUNITY_TOKEN_LAUNCHER })
    ).toThrow(/columbus-5 launcher/)
  })

  it('Playwright Vite bake prefers local pins; smoke falls back to columbus-5', () => {
    expect(communityTaxViteEnvForPlaywright(LOCAL)).toEqual({
      VITE_COMMUNITY_TAX_CODE_ID: '42',
      VITE_COMMUNITY_TOKEN_LAUNCHER: LOCAL.VITE_COMMUNITY_TOKEN_LAUNCHER,
    })
    expect(communityTaxViteEnvForPlaywright({})).toEqual({
      VITE_COMMUNITY_TAX_CODE_ID: MAINNET_COMMUNITY_TAX_CODE_ID,
      VITE_COMMUNITY_TOKEN_LAUNCHER: MAINNET_COMMUNITY_TOKEN_LAUNCHER,
    })
  })
})
