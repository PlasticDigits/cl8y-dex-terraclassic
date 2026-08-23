import { describe, expect, it } from 'vitest'
import { buildValidatedCreateArgs, type CreateTokenFormDraft } from './communityTaxCreateForm'

const WALLET = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const OTHER = 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0'

function draft(over: Partial<CreateTokenFormDraft> = {}): CreateTokenFormDraft {
  return {
    name: 'Demo',
    symbol: 'demo',
    decimals: '6',
    buyPercent: '0',
    sellPercent: '0',
    transferPercent: '0',
    treasury: WALLET,
    manager: WALLET,
    skus: [],
    mintCapHuman: '',
    sinks: [{ kind: 'treasury', addr: '', percent: '100' }],
    exemptList: '',
    maxBuyPercent: '0',
    maxSellPercent: '0',
    maxTransferPercent: '0',
    autolpThresholdHuman: '',
    autolpRecipient: WALLET,
    maxWalletHuman: '',
    cooldownBlocks: '0',
    tradingEnabled: false,
    autolpCodeId: 11613,
    ...over,
  }
}

describe('communityTaxCreateForm (#604 #605)', () => {
  it('P6: symbol demo is submitted uppercase; name case preserved', () => {
    const r = buildValidatedCreateArgs(draft())
    expect(r.ok).toBe(true)
    if (r.ok) {
      expect(r.args.symbol).toBe('DEMO')
      expect(r.args.name).toBe('Demo')
    }
  })

  it('P11: free create omits SKU payloads', () => {
    const r = buildValidatedCreateArgs(draft({ transferPercent: '1.00' }))
    expect(r.ok).toBe(true)
    if (r.ok) {
      expect(r.args.features).toEqual([])
      expect(r.args.transferBps).toBeUndefined()
      expect(r.args.sinks).toBeUndefined()
      expect(r.args.launchGuards).toBeUndefined()
    }
  })

  it('P6: transfer_tax encodes 1.00% as 100 bps', () => {
    const r = buildValidatedCreateArgs(draft({ skus: ['transfer_tax'], transferPercent: '1.00' }))
    expect(r.ok).toBe(true)
    if (r.ok) expect(r.args.transferBps).toBe(100)
  })

  it('P5: combined 10+10+10% is rejected', () => {
    const r = buildValidatedCreateArgs(
      draft({
        skus: ['transfer_tax'],
        buyPercent: '10.00',
        sellPercent: '10.00',
        transferPercent: '10.00',
      })
    )
    expect(r.ok).toBe(false)
  })

  it('P8/P9: sinks 70+30 ok; 50+50+1 reject', () => {
    const ok = buildValidatedCreateArgs(
      draft({
        skus: ['split_router'],
        sinks: [
          { kind: 'treasury', addr: '', percent: '70.00' },
          { kind: 'burn', addr: '', percent: '30.00' },
        ],
      })
    )
    expect(ok.ok).toBe(true)
    if (ok.ok) expect(ok.args.sinks?.map((s) => s.bps)).toEqual([7000, 3000])

    const bad = buildValidatedCreateArgs(
      draft({
        skus: ['split_router'],
        sinks: [
          { kind: 'treasury', addr: '', percent: '50' },
          { kind: 'burn', addr: '', percent: '50' },
          { kind: 'auto_lp', addr: '', percent: '1' },
        ],
      })
    )
    expect(bad.ok).toBe(false)
  })

  it('P13: AutoLP SKU without code_id is blocked', () => {
    const r = buildValidatedCreateArgs(draft({ skus: ['auto_v2_lp'], autolpCodeId: null }))
    expect(r.ok).toBe(false)
  })

  it('P16/P17: variable max must be ≥ current', () => {
    const ok = buildValidatedCreateArgs(
      draft({
        skus: ['variable_rates'],
        buyPercent: '5.00',
        maxBuyPercent: '10.00',
        maxSellPercent: '0',
        maxTransferPercent: '0',
      })
    )
    expect(ok.ok).toBe(true)
    if (ok.ok) expect(ok.args.maxBuyBps).toBe(1000)

    const bad = buildValidatedCreateArgs(
      draft({
        skus: ['variable_rates'],
        buyPercent: '5.00',
        maxBuyPercent: '4.00',
        maxSellPercent: '0',
        maxTransferPercent: '0',
      })
    )
    expect(bad.ok).toBe(false)
  })

  it('P18: launch guards default trading off', () => {
    const r = buildValidatedCreateArgs(draft({ skus: ['launch_guards'], cooldownBlocks: '10' }))
    expect(r.ok).toBe(true)
    if (r.ok) {
      expect(r.args.launchGuards?.trading_enabled).toBe(false)
      expect(r.args.launchGuards?.cooldown_blocks).toBe(10)
    }
  })

  it('A12: max wallet human uses create decimals', () => {
    const six = buildValidatedCreateArgs(draft({ skus: ['launch_guards'], maxWalletHuman: '1', decimals: '6' }))
    const eighteen = buildValidatedCreateArgs(draft({ skus: ['launch_guards'], maxWalletHuman: '1', decimals: '18' }))
    expect(six.ok && eighteen.ok).toBe(true)
    if (six.ok && eighteen.ok) {
      expect(six.args.launchGuards?.max_wallet).toBe('1000000')
      expect(eighteen.args.launchGuards?.max_wallet).toBe('1000000000000000000')
    }
  })

  it('P10: wallet sink requires bech32', () => {
    const r = buildValidatedCreateArgs(
      draft({
        skus: ['split_router'],
        sinks: [{ kind: 'wallet', addr: 'not-an-addr', percent: '100.00' }],
      })
    )
    expect(r.ok).toBe(false)
  })

  it('P14: exempt list accepts a valid EOA', () => {
    const r = buildValidatedCreateArgs(draft({ skus: ['exemption_directory'], exemptList: OTHER }))
    expect(r.ok).toBe(true)
    if (r.ok) expect(r.args.initialExempt).toEqual([OTHER])
  })
})
