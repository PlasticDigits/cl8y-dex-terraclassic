import { describe, expect, it } from 'vitest'
import {
  applyExtraDebitSellCap,
  BUY_TAX_HINT,
  classifyCommunityTaxQueryError,
  communityTaxExecuteUsesRouter,
  communityTaxRouteHint,
  COMMUNITY_TAX_SCOPE_COPY,
  declaredRawForMax,
  effectiveExtraDebitSellBps,
  extraDebitFromDeclared,
  extraDebitMaxDeclaredRaw,
  extraDebitSellBpsForExecute,
  extraDebitSellHuman,
  extraDebitSubmitGate,
  maxDeclaredForExtraDebitSell,
  parseCommunityTaxSellBps,
  parseUintString,
  sellDebitExceedsBalance,
  SELL_TAX_EXTRA_HINT,
} from './taxPreviewMaxSpend'
import { toRawAmount } from './formatAmount'

describe('taxPreviewMaxSpend (#593 extra-debit sell)', () => {
  it('reduces max so debit fits in balance (5% sell)', () => {
    const balance = 1_000_000n
    const declared = maxDeclaredForExtraDebitSell(balance, 500)
    const tax = (declared * 500n) / 10_000n
    expect(declared + tax).toBeLessThanOrEqual(balance)
    expect(declared).toBeLessThan(balance)
  })

  it('zero sell bps keeps full balance', () => {
    expect(maxDeclaredForExtraDebitSell(99n, 0)).toBe(99n)
    expect(applyExtraDebitSellCap(99n, 0)).toBe(99n)
    expect(applyExtraDebitSellCap(99n, null)).toBe(99n)
  })

  it('human max is not the raw 100% balance when taxed', () => {
    expect(extraDebitSellHuman('1000000', 6, 500)).not.toBe('1')
    expect(SELL_TAX_EXTRA_HINT).toBe('Sell tax extra')
  })

  it('does not offer 100% of a taxed sell (abuse = self-DoS)', () => {
    const balance = 10_000_000n
    expect(applyExtraDebitSellCap(balance, 2500)).toBeLessThan(balance)
  })

  it('#609 manager-exempt skips extra-debit; unknown stays fail-closed', () => {
    expect(effectiveExtraDebitSellBps(500, true)).toBe(0)
    expect(effectiveExtraDebitSellBps(500, false)).toBe(500)
    expect(effectiveExtraDebitSellBps(500, null)).toBe(500)
    expect(effectiveExtraDebitSellBps(500, undefined)).toBe(500)
    expect(effectiveExtraDebitSellBps(null, true)).toBeNull()
    expect(applyExtraDebitSellCap(10_000_000n, effectiveExtraDebitSellBps(500, true))).toBe(10_000_000n)
  })

  it('router hops extra-debit the trader and disclose tax (#607 option 2)', () => {
    expect(communityTaxExecuteUsesRouter(1)).toBe(false)
    expect(communityTaxExecuteUsesRouter(2)).toBe(true)
    expect(communityTaxExecuteUsesRouter(1, true)).toBe(true)
    expect(extraDebitSellBpsForExecute(500, false)).toBe(500)
    expect(extraDebitSellBpsForExecute(500, true)).toBe(500)
    expect(communityTaxRouteHint({ payIsTax: true, usesRouter: false, sellBps: 500 })).toBe(SELL_TAX_EXTRA_HINT)
    expect(communityTaxRouteHint({ payIsTax: true, usesRouter: true, sellBps: 500 })).toBe(SELL_TAX_EXTRA_HINT)
    expect(communityTaxRouteHint({ payIsTax: false, receiveIsTax: true, usesRouter: true })).toBe(BUY_TAX_HINT)
    expect(COMMUNITY_TAX_SCOPE_COPY).toBe('Buy/sell tax applies on every listed-pair swap.')
  })
})

describe('extra-debit submit gate (#1267)', () => {
  it('T1: declared = full balance with sell_bps 500 exceeds debit room', () => {
    const balance = 1_050_000n
    const declared = 1_050_000n
    const debit = extraDebitFromDeclared(declared, 500)
    expect(debit).toBeGreaterThan(balance)
    expect(sellDebitExceedsBalance({ declaredRaw: declared, balanceRaw: balance, debitRaw: debit })).toBe(true)
    const gate = extraDebitSubmitGate({
      declaredRaw: declared,
      balanceRaw: balance,
      debitRaw: debit,
      sellBps: 500,
      extraDebitUnresolved: false,
      isNativePay: false,
    })
    expect(gate.insufficientBalance).toBe(true)
    expect(gate.blockSubmit).toBe(true)
  })

  it('T2: Max declared 1_000_000 has preview debit 1_050_000', () => {
    const balance = 1_050_000n
    const declared = maxDeclaredForExtraDebitSell(balance, 500)
    expect(declared).toBe(1_000_000n)
    expect(extraDebitFromDeclared(declared, 500)).toBe(1_050_000n)
    expect(extraDebitFromDeclared(declared, 500) <= balance).toBe(true)
    const gate = extraDebitSubmitGate({
      declaredRaw: declared,
      balanceRaw: balance,
      debitRaw: extraDebitFromDeclared(declared, 500),
      sellBps: 500,
      extraDebitUnresolved: false,
      isNativePay: false,
    })
    expect(gate.blockSubmit).toBe(false)
  })

  it('T3: Max human round-trip stays debit ≤ balance', () => {
    const balance = 1_050_000n
    const human = extraDebitSellHuman(balance.toString(), 6, 500)
    const roundTrip = BigInt(toRawAmount(human, 6))
    expect(roundTrip).toBeLessThanOrEqual(maxDeclaredForExtraDebitSell(balance, 500))
    expect(extraDebitFromDeclared(roundTrip, 500)).toBeLessThanOrEqual(balance)
    expect(roundTrip).toBe(extraDebitMaxDeclaredRaw(balance, 500, 6))
  })

  it('T4: pair-direct and router both keep extra-debit bps (R607-7)', () => {
    expect(extraDebitSellBpsForExecute(500, false)).toBe(500)
    expect(extraDebitSellBpsForExecute(500, true)).toBe(500)
    const full = extraDebitSubmitGate({
      declaredRaw: 1_050_000n,
      balanceRaw: 1_050_000n,
      debitRaw: null,
      sellBps: extraDebitSellBpsForExecute(500, true),
      extraDebitUnresolved: false,
      isNativePay: false,
    })
    expect(full.blockSubmit).toBe(true)
  })

  it('T5: live GetConfig sell_bps is independent of catalog pin / code_id', () => {
    expect(parseCommunityTaxSellBps({ sell_bps: 500 })).toEqual({ kind: 'tax', sellBps: 500 })
    expect(parseCommunityTaxSellBps({ sell_bps: 500, code_id: 11666 })).toEqual({ kind: 'tax', sellBps: 500 })
    expect(parseCommunityTaxSellBps({})).toEqual({ kind: 'not_tax' })
    expect(parseCommunityTaxSellBps({ sell_bps: 'nope' })).toEqual({ kind: 'unresolved' })
  })

  it('T6/T7: manager skip known true unlocks 100%; unknown stays capped', () => {
    expect(effectiveExtraDebitSellBps(500, true)).toBe(0)
    expect(
      extraDebitSubmitGate({
        declaredRaw: 1_050_000n,
        balanceRaw: 1_050_000n,
        debitRaw: null,
        sellBps: extraDebitSellBpsForExecute(effectiveExtraDebitSellBps(500, true), false),
        extraDebitUnresolved: false,
        isNativePay: false,
      }).blockSubmit
    ).toBe(false)
    expect(
      extraDebitSubmitGate({
        declaredRaw: 1_050_000n,
        balanceRaw: 1_050_000n,
        debitRaw: null,
        sellBps: extraDebitSellBpsForExecute(effectiveExtraDebitSellBps(500, null), false),
        extraDebitUnresolved: false,
        isNativePay: false,
      }).blockSubmit
    ).toBe(true)
  })

  it('AC1: Honest LCD debit === declared === balance with sell_bps 500 still blocks', () => {
    const balance = 1_050_000n
    const declared = 1_050_000n
    expect(extraDebitFromDeclared(declared, 500)).toBeGreaterThan(balance)
    const gate = extraDebitSubmitGate({
      declaredRaw: declared,
      balanceRaw: balance,
      debitRaw: declared,
      sellBps: 500,
      extraDebitUnresolved: false,
      isNativePay: false,
    })
    expect(gate.insufficientBalance).toBe(true)
    expect(gate.blockSubmit).toBe(true)
  })

  it('LCD debit larger than local extra-debit floor still blocks', () => {
    const declared = 1_000_000n
    const balance = 1_050_000n
    const localFloor = extraDebitFromDeclared(declared, 500)
    expect(localFloor).toBe(1_050_000n)
    const lcdDebit = 1_200_000n
    expect(lcdDebit).toBeGreaterThan(localFloor)
    const gate = extraDebitSubmitGate({
      declaredRaw: declared,
      balanceRaw: balance,
      debitRaw: lcdDebit,
      sellBps: 500,
      extraDebitUnresolved: false,
      isNativePay: false,
    })
    expect(gate.insufficientBalance).toBe(true)
    expect(gate.blockSubmit).toBe(true)
  })

  it('Honest LCD debit === declared with sell_bps 0 allows amount ≤ balance', () => {
    const gate = extraDebitSubmitGate({
      declaredRaw: 1_050_000n,
      balanceRaw: 1_050_000n,
      debitRaw: 1_050_000n,
      sellBps: 0,
      extraDebitUnresolved: false,
      isNativePay: false,
    })
    expect(gate.insufficientBalance).toBe(false)
    expect(gate.blockSubmit).toBe(false)
  })

  it('T8/T11: sell_bps 0 and native pay stay amount ≤ balance', () => {
    expect(
      extraDebitSubmitGate({
        declaredRaw: 1_050_000n,
        balanceRaw: 1_050_000n,
        debitRaw: null,
        sellBps: 0,
        extraDebitUnresolved: false,
        isNativePay: false,
      }).blockSubmit
    ).toBe(false)
    expect(
      extraDebitSubmitGate({
        declaredRaw: 1_050_000n,
        balanceRaw: 1_050_000n,
        debitRaw: null,
        sellBps: null,
        extraDebitUnresolved: false,
        isNativePay: true,
      }).blockSubmit
    ).toBe(false)
  })

  it('T12: classic amount > balance still insufficient', () => {
    const gate = extraDebitSubmitGate({
      declaredRaw: 2n,
      balanceRaw: 1n,
      debitRaw: null,
      sellBps: null,
      extraDebitUnresolved: false,
      isNativePay: false,
    })
    expect(gate.insufficientBalance).toBe(true)
    expect(gate.blockSubmit).toBe(true)
  })

  it('T13: unresolved tax detection fail-closes without assuming 0-tax', () => {
    const gate = extraDebitSubmitGate({
      declaredRaw: 1_000n,
      balanceRaw: 1_000n,
      debitRaw: null,
      sellBps: null,
      extraDebitUnresolved: true,
      isNativePay: false,
    })
    expect(gate.insufficientBalance).toBe(false)
    expect(gate.blockSubmit).toBe(true)
  })

  it('A11: hostile non-numeric debit does not throw', () => {
    expect(parseUintString('not-a-number')).toBeNull()
    expect(parseUintString({})).toBeNull()
    expect(
      declaredRawForMax({ balanceRaw: 1_050_000n, debitPreview: { declared: 'x', debit: 'y' }, sellBps: 500 })
    ).toBe(maxDeclaredForExtraDebitSell(1_050_000n, 500))
  })

  it('unknown-query errors are honest CW20', () => {
    expect(classifyCommunityTaxQueryError(new Error('unknown variant `get_config`'))).toBe('not_tax')
    expect(classifyCommunityTaxQueryError(new Error('LCD request timed out after 10000ms'))).toBe('unresolved')
  })
})
