import { describe, expect, it } from 'vitest'
import {
  buildCreateTokenInvoice,
  buildEnableFeatureInvoice,
  buildSettingsBatchInvoice,
  settingsBatchIsEmpty,
  uniqueCommunityTaxSkus,
} from './communityTaxInvoice'

const LAUNCHER = 'terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz'
const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const TOKEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const MANAGER = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

function decodeHook(b64: string): Record<string, unknown> {
  return JSON.parse(atob(b64)) as Record<string, unknown>
}

describe('communityTaxInvoice (#593)', () => {
  it('paid create invoices N × 50 UST1 to the env launcher', () => {
    const invoice = buildCreateTokenInvoice({
      launcher: LAUNCHER,
      ust1: UST1,
      args: {
        name: 'Demo',
        symbol: 'DEMO',
        decimals: 6,
        initialBalances: [],
        manager: MANAGER,
        treasury: MANAGER,
        buyBps: 100,
        sellBps: 100,
        features: ['transfer_tax', 'variable_rates'],
      },
    })
    expect(invoice.payee).toBe(LAUNCHER)
    expect(invoice.invoiceToken).toBe(UST1)
    expect(invoice.invoiceAmount).toBe('100000000')
    const hook = decodeHook(invoice.hookMsg)
    expect(hook).toHaveProperty('create_token')
    const ct = hook.create_token as { max_buy_bps: number; max_sell_bps: number; max_transfer_bps: number }
    expect(ct.max_buy_bps + ct.max_sell_bps + ct.max_transfer_bps).toBeLessThanOrEqual(2500)
    expect(JSON.stringify(hook)).not.toContain('increase_allowance')
  })

  it('create unique-sets duplicate SKUs so the invoice is charged once (#606)', () => {
    expect(uniqueCommunityTaxSkus(['transfer_tax', 'transfer_tax', 'variable_rates'])).toEqual([
      'transfer_tax',
      'variable_rates',
    ])
    const invoice = buildCreateTokenInvoice({
      launcher: LAUNCHER,
      ust1: UST1,
      args: {
        name: 'Demo',
        symbol: 'DEMO',
        decimals: 6,
        initialBalances: [],
        manager: MANAGER,
        treasury: MANAGER,
        buyBps: 100,
        sellBps: 100,
        features: ['transfer_tax', 'transfer_tax'],
      },
    })
    expect(invoice.invoiceAmount).toBe('50000000')
    expect((decodeHook(invoice.hookMsg).create_token as { features: string[] }).features).toEqual(['transfer_tax'])
  })

  it('Enable Feature invoices 50 UST1 to the launcher, not a URL payee', () => {
    const invoice = buildEnableFeatureInvoice({
      launcher: LAUNCHER,
      ust1: UST1,
      token: TOKEN,
      sku: 'transfer_tax',
    })
    expect(invoice.payee).toBe(LAUNCHER)
    expect(invoice.invoiceAmount).toBe('50000000')
    expect(decodeHook(invoice.hookMsg)).toEqual({
      enable_feature: { token: TOKEN, sku: 'transfer_tax' },
    })
  })

  it('settings Save is a flat 50 UST1 to the token (not per field)', () => {
    const one = buildSettingsBatchInvoice({
      token: TOKEN,
      ust1: UST1,
      settings: { buy_bps: 200 },
    })
    const many = buildSettingsBatchInvoice({
      token: TOKEN,
      ust1: UST1,
      settings: { buy_bps: 200, sell_bps: 300, treasury: MANAGER },
    })
    expect(one.payee).toBe(TOKEN)
    expect(one.invoiceAmount).toBe(many.invoiceAmount)
    expect(one.invoiceAmount).toBe('50000000')
    expect(settingsBatchIsEmpty({})).toBe(true)
    expect(settingsBatchIsEmpty({ buy_bps: 1 })).toBe(false)
  })

  it('P6/P7: transfer_bps only when transfer_tax SKU is on (#605)', () => {
    const withSku = buildCreateTokenInvoice({
      launcher: LAUNCHER,
      ust1: UST1,
      args: {
        name: 'Demo',
        symbol: 'DEMO',
        decimals: 6,
        initialBalances: [],
        manager: MANAGER,
        treasury: MANAGER,
        buyBps: 0,
        sellBps: 0,
        features: ['transfer_tax'],
        transferBps: 100,
      },
    })
    const ct = decodeHook(withSku.hookMsg).create_token as { transfer_bps?: number; features: string[] }
    expect(ct.transfer_bps).toBe(100)
    expect(ct.features).toContain('transfer_tax')

    const leftover = buildCreateTokenInvoice({
      launcher: LAUNCHER,
      ust1: UST1,
      args: {
        name: 'Demo',
        symbol: 'DEMO',
        decimals: 6,
        initialBalances: [],
        manager: MANAGER,
        treasury: MANAGER,
        buyBps: 0,
        sellBps: 0,
        features: [],
        transferBps: 100,
      },
    })
    const dropped = decodeHook(leftover.hookMsg).create_token as { transfer_bps?: number }
    expect(dropped.transfer_bps).toBeUndefined()
  })

  it('P8: sinks encode percent→bps and A1 snapshot 2.50% → 250', () => {
    const invoice = buildCreateTokenInvoice({
      launcher: LAUNCHER,
      ust1: UST1,
      args: {
        name: 'Demo',
        symbol: 'DEMO',
        decimals: 6,
        initialBalances: [],
        manager: MANAGER,
        treasury: MANAGER,
        buyBps: 250,
        sellBps: 0,
        features: ['split_router'],
        sinks: [
          { kind: 'treasury', bps: 7000 },
          { kind: 'burn', bps: 3000 },
        ],
      },
    })
    const ct = decodeHook(invoice.hookMsg).create_token as {
      buy_bps: number
      sinks: { bps: number }[]
    }
    expect(ct.buy_bps).toBe(250)
    expect(ct.sinks.map((s) => s.bps)).toEqual([7000, 3000])
  })
})
