/**
 * Retail copy for DEX-routed invoice pay (GitLab #595).
 * CTA is Pay / Enable — never “Swap” as the primary action.
 */

export const PAY_INVOICE_NO_ROUTE = 'No route'
export const PAY_INVOICE_INSUFFICIENT = 'Insufficient balance'
export const PAY_INVOICE_WRAP_UNAVAILABLE = 'Wrap config unavailable'
export const PAY_INVOICE_INVALID_PAYEE = 'Invalid payee'
export const PAY_INVOICE_INVALID_INVOICE = 'Invalid invoice'
export const PAY_INVOICE_CALCULATING = 'Calculating'

export function payInvoiceSummaryLine(input: {
  payHuman: string
  paySymbol: string
  invoiceHuman: string
  invoiceSymbol: string
  routed: boolean
}): string {
  if (!input.routed) {
    return `You pay ${input.invoiceHuman} ${input.invoiceSymbol}`
  }
  return `You pay ~${input.payHuman} ${input.paySymbol} (incl. DEX swap) → ${input.invoiceHuman} ${input.invoiceSymbol} fee`
}

export function payInvoiceCtaLabel(kind: 'pay' | 'enable' = 'pay'): string {
  return kind === 'enable' ? 'Enable' : 'Pay'
}
