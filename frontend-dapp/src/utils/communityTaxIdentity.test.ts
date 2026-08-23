import { describe, expect, it } from 'vitest'
import {
  CONNECTED_WALLET_HELPER,
  NOT_CONNECTED_WALLET_HELPER,
  autofillConnectedWallet,
  parseTokenDecimals,
  parseTokenName,
  parseTokenSymbol,
  walletOwnershipHelper,
} from './communityTaxIdentity'

const WALLET = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const OTHER = 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0'

describe('communityTaxIdentity (#604)', () => {
  it('P1: accepts decimals 6 and 18', () => {
    expect(parseTokenDecimals('6')).toEqual({ ok: true, value: 6 })
    expect(parseTokenDecimals('18')).toEqual({ ok: true, value: 18 })
  })

  it('P2: rejects decimals 0 / 5 / 19 / 6.5 / empty / -1', () => {
    for (const raw of ['0', '5', '19', '6.5', '', '-1', 'NaN', '1e1']) {
      expect(parseTokenDecimals(raw).ok).toBe(false)
    }
  })

  it('P3/P6: name Demo and symbol demo → DEMO', () => {
    expect(parseTokenName('Demo')).toEqual({ ok: true, value: 'Demo' })
    expect(parseTokenName('  DeMo  ')).toEqual({ ok: true, value: 'DeMo' })
    expect(parseTokenSymbol('demo')).toEqual({ ok: true, value: 'DEMO' })
    expect(parseTokenSymbol('DeMo')).toEqual({ ok: true, value: 'DEMO' })
  })

  it('P4: rejects spaced / punctuated / unicode / short names', () => {
    for (const raw of ['My Token', 'Demo!', '🚀', '', 'ab', 'x y', '<script>']) {
      expect(parseTokenName(raw).ok).toBe(false)
    }
  })

  it('P5: rejects hyphen / underscore / 1-char / overlong symbols', () => {
    for (const raw of ['DE-MO', 'demo_1', 'D', 'TOOLONGSYMBOLX', 'AB']) {
      expect(parseTokenSymbol(raw).ok).toBe(false)
    }
  })

  it('A2/A3/A5: rejects RTL / zero-width / long / HTML names', () => {
    expect(parseTokenName(`Demo\u200b`).ok).toBe(false)
    expect(parseTokenName('ديمو').ok).toBe(false)
    expect(parseTokenName('A'.repeat(51)).ok).toBe(false)
    expect(parseTokenName('<img src=x>').ok).toBe(false)
  })

  it('wallet helper is connected vs not connected (bech32-normalized)', () => {
    expect(walletOwnershipHelper(WALLET, WALLET)).toBe(CONNECTED_WALLET_HELPER)
    expect(walletOwnershipHelper(WALLET.toUpperCase(), WALLET)).toBe(CONNECTED_WALLET_HELPER)
    expect(walletOwnershipHelper(OTHER, WALLET)).toBe(NOT_CONNECTED_WALLET_HELPER)
    expect(walletOwnershipHelper('', WALLET)).toBe(NOT_CONNECTED_WALLET_HELPER)
    expect(walletOwnershipHelper(WALLET, null)).toBe(NOT_CONNECTED_WALLET_HELPER)
  })

  it('autofill only fills empty fields', () => {
    expect(autofillConnectedWallet('', WALLET)).toBe(WALLET)
    expect(autofillConnectedWallet('  ', WALLET)).toBe(WALLET)
    expect(autofillConnectedWallet(OTHER, WALLET)).toBe(OTHER)
    expect(autofillConnectedWallet('', null)).toBe('')
  })
})
