import { describe, expect, it, vi } from 'vitest'
import { tokenAssetInfo } from '@/types'
import { applySlippagePercentCeiling } from '@/utils/rawAmountMath'
import {
  PAY_INVOICE_INSUFFICIENT,
  PAY_INVOICE_INVALID_PAYEE,
  PAY_INVOICE_NO_ROUTE,
  defaultPayToken,
  quotePayInvoice,
  resolveInvoicePayee,
  wrapGrossForNetCw20,
  type Invoice,
  type PayInvoiceQuoteDeps,
} from './payInvoice'
import { payInvoiceCtaLabel, payInvoiceSummaryLine } from './payInvoiceCopy'

const UST1 = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const CLUNC = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const PAYEE = 'terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l'
const INVOICE: Invoice = {
  invoiceToken: UST1,
  invoiceAmount: '50000000',
  payee: PAYEE,
  hookMsg: btoa(JSON.stringify({ enable_feature: { sku: 'transfer_tax' } })),
}

function hop(from: string, to: string) {
  return {
    terra_swap: {
      offer_asset_info: tokenAssetInfo(from),
      ask_asset_info: tokenAssetInfo(to),
    },
  }
}

function indexerSolve(from: string, to: string, hops: number) {
  const ops = hops === 1 ? [hop(from, to)] : [hop(from, CLUNC), hop(CLUNC, to)]
  return {
    token_in: from,
    token_out: to,
    hops: ops.map(() => ({
      offer_token: from,
      ask_token: to,
      pair: 'terra1pair',
    })),
    intermediate_tokens: hops === 1 ? [from, to] : [from, CLUNC, to],
    router_operations: ops,
    estimated_amount_out: '50000000',
  }
}

function deps(overrides?: Partial<PayInvoiceQuoteDeps>): PayInvoiceQuoteDeps {
  return {
    getRouteSolve: vi.fn(async (tin, tout) => indexerSolve(tin, tout, 1)),
    reverseSimulate: vi.fn(async () => ({ amount: '100000000' })),
    forwardSimulate: vi.fn(async () => ({ amount: '50000000' })),
    ...overrides,
  }
}

describe('quotePayInvoice (GitLab #595)', () => {
  it('invoice token + sufficient balance → payRaw = invoice, 0 hops', async () => {
    const q = await quotePayInvoice({
      invoice: INVOICE,
      payToken: UST1,
      slippagePercent: 5,
      payTokenBalance: '50000000',
      trader: PAYEE,
    })
    expect(q.status).toBe('ok')
    if (q.status !== 'ok') return
    expect(q.kind).toBe('direct')
    expect(q.payRaw).toBe('50000000')
    expect(q.hopCount).toBe(0)
    expect(q.operations).toEqual([])
    expect(q.minInvoiceOut).toBe('50000000')
  })

  it('invoice token, insufficient balance → disable (does not switch token)', async () => {
    const q = await quotePayInvoice({
      invoice: INVOICE,
      payToken: UST1,
      slippagePercent: 5,
      payTokenBalance: '49999999',
    })
    expect(q).toEqual({ status: 'unavailable', disableReason: PAY_INVOICE_INSUFFICIENT })
  })

  it('invalid payee disables', async () => {
    const q = await quotePayInvoice({
      invoice: { ...INVOICE, payee: 'not-an-address' },
      payToken: UST1,
      slippagePercent: 5,
    })
    expect(q).toEqual({ status: 'unavailable', disableReason: PAY_INVOICE_INVALID_PAYEE })
  })

  it('1-hop reverse-sim then forward-sim out ≥ invoice', async () => {
    const d = deps()
    const q = await quotePayInvoice({ invoice: INVOICE, payToken: CLUNC, slippagePercent: 5, trader: PAYEE }, d)
    expect(q.status).toBe('ok')
    if (q.status !== 'ok') return
    expect(q.kind).toBe('routed')
    expect(q.hopCount).toBe(1)
    expect(q.minInvoiceOut).toBe('50000000')
    expect(q.payRaw).toBe(applySlippagePercentCeiling('100000000', 5))
    expect(d.reverseSimulate).toHaveBeenCalledWith('50000000', expect.any(Array))
    expect(d.forwardSimulate).toHaveBeenCalled()
    expect(d.getRouteSolve).toHaveBeenCalledWith(CLUNC, UST1, '50000000', expect.objectContaining({ trader: PAYEE }))
  })

  it('≥2-hop path uses indexer ops', async () => {
    const d = deps({
      getRouteSolve: vi.fn(async (tin, tout) => indexerSolve(tin, tout, 2)),
    })
    const q = await quotePayInvoice({ invoice: INVOICE, payToken: CLUNC, slippagePercent: 1 }, d)
    expect(q.status).toBe('ok')
    if (q.status !== 'ok') return
    expect(q.hopCount).toBe(2)
  })

  it('slippage 0.5 / 1 / 5 / custom scales max_in; minimum_receive stays invoice', async () => {
    for (const pct of [0.5, 1, 5, 12.5]) {
      const q = await quotePayInvoice({ invoice: INVOICE, payToken: CLUNC, slippagePercent: pct }, deps())
      expect(q.status).toBe('ok')
      if (q.status !== 'ok') return
      expect(q.minInvoiceOut).toBe('50000000')
      expect(q.payRaw).toBe(applySlippagePercentCeiling('100000000', pct))
    }
  })

  it('50% slippage still cannot pay below invoice', async () => {
    const q = await quotePayInvoice({ invoice: INVOICE, payToken: CLUNC, slippagePercent: 50 }, deps())
    expect(q.status).toBe('ok')
    if (q.status !== 'ok') return
    expect(q.minInvoiceOut).toBe(INVOICE.invoiceAmount)
    expect(BigInt(q.payRaw)).toBeGreaterThan(100000000n)
  })

  it('unroutable / gem / solver miss → No route', async () => {
    const d = deps({
      getRouteSolve: vi.fn(async () => {
        throw new Error('no path')
      }),
    })
    const q = await quotePayInvoice({ invoice: INVOICE, payToken: CLUNC, slippagePercent: 5 }, d)
    expect(q).toEqual({ status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE })
  })

  it('forward sim still short after bumps → No route', async () => {
    const d = deps({
      forwardSimulate: vi.fn(async () => ({ amount: '1' })),
    })
    const q = await quotePayInvoice({ invoice: INVOICE, payToken: CLUNC, slippagePercent: 5 }, d)
    expect(q).toEqual({ status: 'unavailable', disableReason: PAY_INVOICE_NO_ROUTE })
  })

  it('tax extra-debit insufficient → fail closed', async () => {
    const q = await quotePayInvoice({
      invoice: INVOICE,
      payToken: UST1,
      slippagePercent: 5,
      payTokenBalance: '50000000',
      taxPreviewExtraDebit: () => '40000000',
    })
    expect(q).toEqual({ status: 'unavailable', disableReason: PAY_INVOICE_INSUFFICIENT })
  })

  it('tax extra-debit raises payRaw on the direct path', async () => {
    const q = await quotePayInvoice({
      invoice: INVOICE,
      payToken: UST1,
      slippagePercent: 5,
      payTokenBalance: '60000000',
      taxPreviewExtraDebit: (intended) => (BigInt(intended) + 1_000_000n).toString(),
    })
    expect(q.status).toBe('ok')
    if (q.status !== 'ok') return
    expect(q.payRaw).toBe('51000000')
  })
})

describe('pay invoice helpers', () => {
  it('resolveInvoicePayee ignores query-string spoof', () => {
    expect(resolveInvoicePayee(INVOICE, '?payee=terra1evil')).toBe(PAYEE)
  })

  it('defaultPayToken prefers invoice token when the wallet holds enough', () => {
    expect(
      defaultPayToken({
        invoiceToken: UST1,
        invoiceAmount: '50000000',
        pickerTokens: [CLUNC, UST1],
        balances: { [UST1]: '50000000', [CLUNC]: '1' },
      })
    ).toBe(UST1)
  })

  it('wrapGrossForNetCw20 inverts mapper fee with ceiling', () => {
    const net = 99_500_000n
    const gross = wrapGrossForNetCw20(net, 50)
    expect(netAfter(gross, 50)).toBeGreaterThanOrEqual(net)
  })

  it('copy: routed summary and Pay/Enable CTAs', () => {
    expect(
      payInvoiceSummaryLine({
        payHuman: '12.5',
        paySymbol: 'cLUNC',
        invoiceHuman: '50',
        invoiceSymbol: 'UST1',
        routed: true,
      })
    ).toBe('You pay ~12.5 cLUNC (incl. DEX swap) → 50 UST1 fee')
    expect(payInvoiceCtaLabel('pay')).toBe('Pay')
    expect(payInvoiceCtaLabel('enable')).toBe('Enable')
  })
})

function netAfter(gross: bigint, feeBps: number): bigint {
  return gross - (gross * BigInt(feeBps)) / 10000n
}
