import { beforeEach, describe, expect, it, vi } from 'vitest'
import { tokenAssetInfo } from '@/types'
import {
  PAY_INVOICE_NO_ROUTE,
  assertPayInvoiceMsgs,
  buildPayInvoiceMsgs,
  payInvoiceBroadcastCount,
  type Invoice,
  type PayInvoiceQuoteOk,
} from './payInvoice'

const { UST1, CLUNC, PAYEE, WALLET, ROUTER, TREASURY } = vi.hoisted(() => ({
  UST1: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
  CLUNC: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
  PAYEE: 'terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l',
  WALLET: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
  ROUTER: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
  TREASURY: 'terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l',
}))

const HOOK = btoa(JSON.stringify({ enable_feature: { sku: 'transfer_tax' } }))
const INVOICE: Invoice = {
  invoiceToken: UST1,
  invoiceAmount: '50000000',
  payee: PAYEE,
  hookMsg: HOOK,
}

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    ROUTER_CONTRACT_ADDRESS: ROUTER,
    TREASURY_CONTRACT_ADDRESS: TREASURY,
    WRAP_MAPPER_CONTRACT_ADDRESS: ROUTER,
    LUNC_C_TOKEN_ADDRESS: CLUNC,
    USTC_C_TOKEN_ADDRESS: UST1,
    isNativeWrapEnabled: () => true,
  }
})

function routedQuote(over: Partial<PayInvoiceQuoteOk> = {}): PayInvoiceQuoteOk {
  return {
    status: 'ok',
    kind: 'routed',
    payToken: CLUNC,
    offerCw20: CLUNC,
    payRaw: '105000000',
    cw20SendAmount: '105000000',
    maxIn: '105000000',
    minInvoiceOut: '50000000',
    operations: [
      {
        terra_swap: {
          offer_asset_info: tokenAssetInfo(CLUNC),
          ask_asset_info: tokenAssetInfo(UST1),
        },
      },
    ],
    routeLabel: 'cLUNC → UST1',
    hopCount: 1,
    ...over,
  }
}

function decode(msg: string): Record<string, unknown> {
  return JSON.parse(atob(msg)) as Record<string, unknown>
}

describe('buildPayInvoiceMsgs (GitLab #595)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('invoice token: one Send, no router', () => {
    const quote: PayInvoiceQuoteOk = {
      ...routedQuote(),
      kind: 'direct',
      payToken: UST1,
      offerCw20: UST1,
      payRaw: '50000000',
      cw20SendAmount: '50000000',
      maxIn: '50000000',
      operations: [],
      hopCount: 0,
      routeLabel: '',
    }
    const msgs = buildPayInvoiceMsgs({
      invoice: INVOICE,
      quote,
      walletAddress: WALLET,
      slippagePercent: 5,
    })
    expect(msgs).toHaveLength(1)
    expect(msgs[0]!.contract).toBe(UST1)
    const send = msgs[0]!.msg.send as { contract: string; amount: string; msg: string }
    expect(send.contract).toBe(PAYEE)
    expect(send.amount).toBe('50000000')
    expect(send.msg).toBe(HOOK)
    expect(JSON.stringify(msgs)).not.toContain('execute_swap_operations')
    expect(payInvoiceBroadcastCount(msgs)).toBe(1)
  })

  it('routed: swap then exact invoice Send; swap to = user not payee', () => {
    const msgs = buildPayInvoiceMsgs({
      invoice: INVOICE,
      quote: routedQuote(),
      walletAddress: WALLET,
      slippagePercent: 5,
    })
    expect(msgs).toHaveLength(2)
    expect(payInvoiceBroadcastCount(msgs)).toBe(1)
    const routerSend = msgs[0]!.msg.send as { contract: string; amount: string; msg: string }
    expect(routerSend.contract).toBe(ROUTER)
    expect(routerSend.amount).toBe('105000000')
    const exec = decode(routerSend.msg).execute_swap_operations as {
      minimum_receive: string
      to: string
    }
    expect(exec.minimum_receive).toBe('50000000')
    expect(exec.to).toBe(WALLET)
    expect(exec.to).not.toBe(PAYEE)

    const payeeSend = msgs[1]!.msg.send as { contract: string; amount: string }
    expect(payeeSend.contract).toBe(PAYEE)
    expect(payeeSend.amount).toBe('50000000')
    expect(payeeSend.amount).not.toBe('50100000')
  })

  it('wrap path: wrap_deposit → router Send → payee Send', () => {
    const quote = routedQuote({
      kind: 'wrap_routed',
      payToken: 'uluna',
      wrap: { denom: 'uluna', grossNative: '110000000' },
    })
    const msgs = buildPayInvoiceMsgs({
      invoice: INVOICE,
      quote,
      walletAddress: WALLET,
      slippagePercent: 5,
    })
    expect(msgs).toHaveLength(3)
    expect(msgs[0]!.msg).toEqual({ wrap_deposit: {} })
    expect(msgs[0]!.coins).toEqual([{ denom: 'uluna', amount: '110000000' }])
    expect(msgs[1]!.contract).toBe(CLUNC)
    expect(msgs[2]!.contract).toBe(UST1)
    expect(payInvoiceBroadcastCount(msgs)).toBe(1)
  })

  it('rejects missing minimum_receive (mutate)', () => {
    const msgs = buildPayInvoiceMsgs({
      invoice: INVOICE,
      quote: routedQuote(),
      walletAddress: WALLET,
      slippagePercent: 5,
    })
    const send = msgs[0]!.msg.send as { msg: string }
    const inner = decode(send.msg)
    delete (inner.execute_swap_operations as { minimum_receive?: string }).minimum_receive
    send.msg = btoa(JSON.stringify(inner))
    expect(() =>
      assertPayInvoiceMsgs(msgs, {
        invoice: INVOICE,
        quote: routedQuote(),
        walletAddress: WALLET,
        slippagePercent: 5,
      })
    ).toThrow(/minimum_receive/)
  })

  it('rejects swap to = payee (overpay theft)', () => {
    const msgs = buildPayInvoiceMsgs({
      invoice: INVOICE,
      quote: routedQuote(),
      walletAddress: WALLET,
      slippagePercent: 5,
    })
    const send = msgs[0]!.msg.send as { msg: string }
    const inner = decode(send.msg)
    ;(inner.execute_swap_operations as { to: string }).to = PAYEE
    send.msg = btoa(JSON.stringify(inner))
    expect(() =>
      assertPayInvoiceMsgs(msgs, {
        invoice: INVOICE,
        quote: routedQuote(),
        walletAddress: WALLET,
        slippagePercent: 5,
      })
    ).toThrow(/must not be the payee/)
  })

  it('rejects fake route (offer mismatch)', () => {
    expect(() =>
      buildPayInvoiceMsgs({
        invoice: INVOICE,
        quote: routedQuote({
          operations: [
            {
              terra_swap: {
                offer_asset_info: tokenAssetInfo(UST1),
                ask_asset_info: tokenAssetInfo(CLUNC),
              },
            },
          ],
        }),
        walletAddress: WALLET,
        slippagePercent: 5,
      })
    ).toThrow(PAY_INVOICE_NO_ROUTE)
  })

  it('rejects payee amount ≠ invoice (swap output 50.1 must not go to payee)', () => {
    const msgs = buildPayInvoiceMsgs({
      invoice: INVOICE,
      quote: routedQuote(),
      walletAddress: WALLET,
      slippagePercent: 5,
    })
    ;(msgs[1]!.msg.send as { amount: string }).amount = '50100000'
    expect(() =>
      assertPayInvoiceMsgs(msgs, {
        invoice: INVOICE,
        quote: routedQuote(),
        walletAddress: WALLET,
        slippagePercent: 5,
      })
    ).toThrow(/equal invoice/)
  })

  it('never emits increase_allowance', () => {
    const msgs = buildPayInvoiceMsgs({
      invoice: INVOICE,
      quote: routedQuote(),
      walletAddress: WALLET,
      slippagePercent: 5,
    })
    expect(JSON.stringify(msgs)).not.toContain('increase_allowance')
  })
})
